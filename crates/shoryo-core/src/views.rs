//! Views the screen draws, computed from the state: prerequisite chains, the map, the path
//! through it and the review marks (`docs/spec/screen.md`, "前提の行", "地図").

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::ids::{DecisionId, QuestionId};
use crate::state::{Decision, NodeRef, Origin, Question, Stamp, Topic};

/// How many names a chain shows at most.
const CHAIN_LENGTH: usize = 3;

/// How many times the columns are re-ordered to reduce crossings.
const ORDERING_SWEEPS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChainLink {
    pub decision: DecisionId,
    pub name: String,
    pub in_review: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Decision,
    Question,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MapNode {
    /// `d:<decision id>`, `q:<question id>` or `r:<index into the rejected records>`.
    pub key: String,
    pub kind: NodeKind,
    /// The decision's short name, the question's text or the rejected option's text.
    pub label: String,
    /// The text of the question the node came from (decisions and rejected options).
    pub question: Option<String>,
    pub question_id: Option<QuestionId>,
    pub round: u32,
    pub in_review: bool,
    pub review_mark: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MapEdge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub dashed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Column {
    pub round: u32,
    pub subject: String,
    /// Node keys from top to bottom.
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Map {
    pub columns: Vec<Column>,
    pub nodes: Vec<MapNode>,
    pub edges: Vec<MapEdge>,
}

impl Topic {
    /// 前提の行: for each direct prerequisite of the question, that decision followed back
    /// through the first prerequisite of the question it was decided in.
    pub fn chains(&self, question: &QuestionId) -> Vec<Vec<ChainLink>> {
        let Some((question, _)) = self.find_question(question) else {
            return Vec::new();
        };
        question
            .premises
            .iter()
            .map(|premise| self.chain_from(premise))
            .collect()
    }

    fn chain_from(&self, start: &DecisionId) -> Vec<ChainLink> {
        let mut chain = Vec::new();
        let mut next = Some(start.clone());
        while let Some(id) = next.take() {
            if chain.len() == CHAIN_LENGTH {
                break;
            }
            let Some(decision) = self.find_decision(&id) else {
                break;
            };
            chain.push(ChainLink {
                name: decision.current().name.clone(),
                in_review: self.is_in_review(&id),
                decision: id,
            });
            next = self
                .origin_question(decision)
                .and_then(|(question, _)| question.premises.first().cloned())
                .filter(|premise| !chain.iter().any(|link| &link.decision == premise));
        }
        chain
    }

    /// 代決の印: the decision came from an answer sent with the LLM's stamp, and has not been
    /// revised since (a revision is answered again by the person).
    pub fn decided_pre_approved(&self, id: &DecisionId) -> bool {
        self.find_decision(id).is_some_and(|decision| {
            !decision.was_revised()
                && self
                    .origin_question(decision)
                    .is_some_and(|(question, _)| question.answer.stamp == Some(Stamp::PreApproved))
        })
    }

    /// 地図: the decisions, the current round's questions and the rejected options, by round.
    pub fn map(&self) -> Map {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let marked = self.review_marked();

        for decision in &self.records.decisions {
            let origin = self.origin_question(decision);
            let key = decision_key(&decision.id);
            nodes.push(MapNode {
                kind: NodeKind::Decision,
                label: decision.current().name.clone(),
                question: origin.map(|(question, _)| question.text.clone()),
                question_id: origin.map(|(question, _)| question.id.clone()),
                round: self.decision_round(decision),
                in_review: self.is_in_review(&decision.id),
                review_mark: marked.contains(&NodeRef::Decision(decision.id.clone())),
                key: key.clone(),
            });
            if let Some((question, _)) = origin {
                edges.extend(premise_edges(question, &key, false));
            }
        }

        if let Some(current) = self.current_round() {
            for question in &current.questions {
                let key = question_key(&question.id);
                nodes.push(MapNode {
                    kind: NodeKind::Question,
                    label: question.text.clone(),
                    question: None,
                    question_id: Some(question.id.clone()),
                    round: current.number,
                    in_review: false,
                    review_mark: marked.contains(&NodeRef::Question(question.id.clone())),
                    key: key.clone(),
                });
                edges.extend(premise_edges(question, &key, false));
            }
        }

        for (index, rejected) in self.records.rejected.iter().enumerate() {
            let key = format!("r:{index}");
            let origin = self.find_question(&rejected.question);
            nodes.push(MapNode {
                kind: NodeKind::Rejected,
                label: rejected.text.clone(),
                question: origin.map(|(question, _)| question.text.clone()),
                question_id: Some(rejected.question.clone()),
                round: origin.map_or(0, |(_, round)| round),
                in_review: false,
                review_mark: false,
                key: key.clone(),
            });
            if let Some((question, _)) = origin {
                edges.extend(premise_edges(question, &key, true));
            }
        }

        let known: HashSet<&str> = nodes.iter().map(|node| node.key.as_str()).collect();
        edges.retain(|edge| known.contains(edge.from.as_str()));
        let columns = order_columns(self.columns(&nodes), &edges);
        Map {
            columns,
            nodes,
            edges,
        }
    }

    fn columns(&self, nodes: &[MapNode]) -> Vec<Column> {
        self.rounds
            .iter()
            .map(|round| Column {
                round: round.number,
                subject: round.subject.clone(),
                nodes: nodes
                    .iter()
                    .filter(|node| node.round == round.number)
                    .map(|node| node.key.clone())
                    .collect(),
            })
            .collect()
    }

    /// Nodes resting directly on a revised decision, unless confirmed since the revision.
    fn review_marked(&self) -> HashSet<NodeRef> {
        let mut marked = HashSet::new();
        for revision in &self.records.revisions {
            let dependents = self
                .records
                .decisions
                .iter()
                .filter(|decision| {
                    self.origin_question(decision)
                        .is_some_and(|(question, _)| question.premises.contains(&revision.decision))
                })
                .map(|decision| NodeRef::Decision(decision.id.clone()))
                .chain(
                    self.current_round()
                        .into_iter()
                        .flat_map(|round| &round.questions)
                        .filter(|question| question.premises.contains(&revision.decision))
                        .map(|question| NodeRef::Question(question.id.clone())),
                );
            for node in dependents {
                let confirmed = self
                    .rounds
                    .iter()
                    .filter(|round| round.number >= revision.round)
                    .any(|round| round.confirmed.contains(&node));
                if !confirmed {
                    marked.insert(node);
                }
            }
        }
        marked
    }

    fn find_question(&self, id: &QuestionId) -> Option<(&Question, u32)> {
        self.rounds.iter().find_map(|round| {
            round
                .questions
                .iter()
                .find(|question| &question.id == id)
                .map(|question| (question, round.number))
        })
    }

    fn find_decision(&self, id: &DecisionId) -> Option<&Decision> {
        self.records
            .decisions
            .iter()
            .find(|decision| &decision.id == id)
    }

    fn origin_question(&self, decision: &Decision) -> Option<(&Question, u32)> {
        match &decision.origin {
            Origin::Question(question) => self.find_question(question),
            Origin::FixRound(_) => None,
        }
    }

    /// The column a decision sits in: its question's round, or its fix's round.
    fn decision_round(&self, decision: &Decision) -> u32 {
        match &decision.origin {
            Origin::FixRound(round) => *round,
            Origin::Question(_) => self
                .origin_question(decision)
                .map_or_else(|| decision.decided_in(), |(_, round)| round),
        }
    }
}

impl Map {
    pub fn node(&self, key: &str) -> Option<&MapNode> {
        self.nodes.iter().find(|node| node.key == key)
    }

    pub fn edge(&self, from: &str, to: &str) -> Option<&MapEdge> {
        self.edges
            .iter()
            .find(|edge| edge.from == from && edge.to == to)
    }

    /// 道筋: the selected node, every node reachable back through prerequisites, and the
    /// options rejected in the questions along the way.
    pub fn path(&self, selected: &str) -> Vec<String> {
        let mut on_path: Vec<String> = Vec::new();
        let mut stack = vec![selected.to_string()];
        while let Some(key) = stack.pop() {
            if on_path.contains(&key) || self.node(&key).is_none() {
                continue;
            }
            stack.extend(
                self.edges
                    .iter()
                    .filter(|edge| edge.to == key && !edge.dashed)
                    .map(|edge| edge.from.clone()),
            );
            on_path.push(key);
        }
        let questions: HashSet<&QuestionId> = on_path
            .iter()
            .filter_map(|key| self.node(key))
            .filter_map(|node| node.question_id.as_ref())
            .collect();
        let rejected: Vec<String> = self
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Rejected)
            .filter(|node| {
                node.question_id
                    .as_ref()
                    .is_some_and(|q| questions.contains(q))
            })
            .map(|node| node.key.clone())
            .collect();
        on_path.extend(rejected);
        on_path
    }
}

fn decision_key(id: &DecisionId) -> String {
    format!("d:{id}")
}

fn question_key(id: &QuestionId) -> String {
    format!("q:{id}")
}

/// Edges from each prerequisite of `question` to `to`, named after the question.
fn premise_edges<'a>(
    question: &'a Question,
    to: &'a str,
    dashed: bool,
) -> impl Iterator<Item = MapEdge> + 'a {
    question.premises.iter().map(move |premise| MapEdge {
        from: decision_key(premise),
        to: to.to_string(),
        label: question.text.clone(),
        dashed,
    })
}

/// Orders each column by the mean position of its neighbours in the adjacent columns,
/// sweeping left to right and back, which reduces crossings without an exact solution.
fn order_columns(mut columns: Vec<Column>, edges: &[MapEdge]) -> Vec<Column> {
    for sweep in 0..ORDERING_SWEEPS {
        let forward = sweep % 2 == 0;
        let indices: Vec<usize> = if forward {
            (1..columns.len()).collect()
        } else {
            (0..columns.len().saturating_sub(1)).rev().collect()
        };
        for index in indices {
            let position: HashMap<&str, usize> = columns
                .iter()
                .flat_map(|column| column.nodes.iter().enumerate())
                .map(|(row, key)| (key.as_str(), row))
                .collect();
            let neighbours = |key: &str| -> Vec<usize> {
                edges
                    .iter()
                    .filter_map(|edge| {
                        let other = if forward {
                            (edge.to == key).then_some(edge.from.as_str())
                        } else {
                            (edge.from == key).then_some(edge.to.as_str())
                        };
                        other.and_then(|other| position.get(other).copied())
                    })
                    .collect()
            };
            let mut keyed: Vec<(f64, usize, String)> = columns[index]
                .nodes
                .iter()
                .enumerate()
                .map(|(row, key)| {
                    let near = neighbours(key);
                    let centre = if near.is_empty() {
                        row as f64
                    } else {
                        near.iter().sum::<usize>() as f64 / near.len() as f64
                    };
                    (centre, row, key.clone())
                })
                .collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            columns[index].nodes = keyed.into_iter().map(|(_, _, key)| key).collect();
        }
    }
    columns
}
