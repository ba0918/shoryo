//! A round as the agent sends it, the checks it must pass, and how it changes the topic
//! (`docs/spec/server.md`, "ラウンドを出す").

use std::collections::HashSet;
use std::fmt;

use serde::Deserialize;

use crate::ids::{DecisionId, QuestionId};
use crate::state::{
    Answer, Choice, Class, Decision, DecisionChange, DecisionContent, Delegated, Fix, NodeRef,
    Origin, Question, Rejected, ReviewConclusion, Revision, Round, Topic, Undecided,
};

/// A round as the agent writes it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoundInput {
    pub title: Option<String>,
    pub original_request: Option<String>,
    pub subject: String,
    #[serde(default)]
    pub review: bool,
    #[serde(default)]
    pub fixes: Vec<FixInput>,
    #[serde(default)]
    pub questions: Vec<QuestionInput>,
    #[serde(default)]
    pub records: RecordsInput,
    #[serde(default)]
    pub review_conclusions: Vec<ReviewConclusion>,
    #[serde(default)]
    pub confirmed: Vec<NodeRef>,
    pub finished_picture: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixInput {
    pub text: String,
    /// The decision this fix changed, sent with its new content in the records.
    pub decision: Option<DecisionId>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionInput {
    pub id: QuestionId,
    pub text: String,
    pub class: Class,
    pub why_now: String,
    #[serde(default)]
    pub premises: Vec<DecisionId>,
    #[serde(default)]
    pub background: String,
    pub options: Vec<Choice>,
    pub reasks: Option<QuestionId>,
}

/// The records a round carries. Decisions are added or updated by identifier and never
/// removed; each other kind, when present, replaces the stored list of that kind.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordsInput {
    #[serde(default)]
    pub decisions: Vec<DecisionInput>,
    pub not_building: Option<Vec<String>>,
    pub undecided: Option<Vec<Undecided>>,
    pub delegated: Option<Vec<Delegated>>,
    pub rejected: Option<Vec<Rejected>>,
    /// Accepted and ignored: the in-review marks are the person's, set on the screen.
    #[serde(default, rename = "in_review")]
    _in_review: serde::de::IgnoredAny,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionInput {
    pub id: DecisionId,
    pub name: String,
    pub text: String,
    pub decided_by: DecidedBy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecidedBy {
    Question(QuestionId),
    /// A fix in this review round.
    Fix,
}

/// The round was not valid JSON for a round.
#[derive(Debug)]
pub struct InputError(serde_json::Error);

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a valid round: {}", self.0)
    }
}

impl std::error::Error for InputError {}

impl RoundInput {
    pub fn from_json(json: &str) -> Result<Self, InputError> {
        serde_json::from_str(json).map_err(InputError)
    }
}

/// Why a round was not accepted. Returned to the agent, never shown to the person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoundRefusal {
    PreviousRoundNotSubmitted {
        round: u32,
    },
    RecommendedCount {
        question: QuestionId,
        count: usize,
    },
    MissingConsequence {
        question: QuestionId,
        option: usize,
    },
    UnknownPremise {
        question: QuestionId,
        decision: DecisionId,
    },
    QuestionIdReused {
        question: QuestionId,
    },
}

impl fmt::Display for RoundRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreviousRoundNotSubmitted { round } => {
                write!(f, "round {round} has not been sent yet")
            }
            Self::RecommendedCount { question, count } => write!(
                f,
                "question {question} has {count} recommended options; it needs exactly one"
            ),
            Self::MissingConsequence { question, option } => write!(
                f,
                "option {option} of question {question} has no consequence text"
            ),
            Self::UnknownPremise { question, decision } => write!(
                f,
                "question {question} rests on decision {decision}, which is not in the records"
            ),
            Self::QuestionIdReused { question } => {
                write!(f, "question id {question} is already used in this topic")
            }
        }
    }
}

impl std::error::Error for RoundRefusal {}

impl Topic {
    /// Checks a round and, when it passes, makes it the current round.
    ///
    /// The state is changed in place only after every check has passed, so a refused round
    /// leaves the topic as it was; the server owns the one copy of the state.
    pub fn apply_round(&mut self, input: RoundInput) -> Result<u32, RoundRefusal> {
        self.check_round(&input)?;
        let number = self.rounds.last().map_or(1, |round| round.number + 1);

        if let Some(title) = input.title {
            self.title = title;
        }
        if let Some(request) = input.original_request {
            self.original_request = request;
        }
        if let Some(picture) = input.finished_picture {
            self.finished_picture = Some(picture);
        }
        self.ended = false;

        let changed: Vec<DecisionId> = self.apply_records(number, input.records);
        let fixes = input
            .fixes
            .into_iter()
            .map(|fix| self.fix_with_change(number, fix, &changed))
            .collect();
        self.records.in_review.retain(|mark| {
            !input
                .review_conclusions
                .iter()
                .any(|conclusion| conclusion.decision == mark.decision)
        });

        self.rounds.push(Round {
            number,
            subject: input.subject,
            review: input.review,
            fixes,
            questions: input.questions.into_iter().map(new_question).collect(),
            review_conclusions: input.review_conclusions,
            confirmed: input.confirmed,
            asks: Vec::new(),
            submitted: false,
            sent_at: None,
            sent_as: None,
        });
        Ok(number)
    }

    fn check_round(&self, input: &RoundInput) -> Result<(), RoundRefusal> {
        if let Some(last) = self.rounds.last()
            && !last.submitted
        {
            return Err(RoundRefusal::PreviousRoundNotSubmitted { round: last.number });
        }

        let mut used: HashSet<&QuestionId> = self
            .rounds
            .iter()
            .flat_map(|round| &round.questions)
            .map(|question| &question.id)
            .collect();
        let known: HashSet<&DecisionId> = self
            .records
            .decisions
            .iter()
            .map(|decision| &decision.id)
            .chain(input.records.decisions.iter().map(|decision| &decision.id))
            .collect();

        for question in &input.questions {
            if !used.insert(&question.id) {
                return Err(RoundRefusal::QuestionIdReused {
                    question: question.id.clone(),
                });
            }
            let count = question.options.iter().filter(|o| o.recommended).count();
            if count != 1 {
                return Err(RoundRefusal::RecommendedCount {
                    question: question.id.clone(),
                    count,
                });
            }
            if let Some(option) = question
                .options
                .iter()
                .position(|o| o.consequence.trim().is_empty())
            {
                return Err(RoundRefusal::MissingConsequence {
                    question: question.id.clone(),
                    option,
                });
            }
            if let Some(decision) = question.premises.iter().find(|d| !known.contains(d)) {
                return Err(RoundRefusal::UnknownPremise {
                    question: question.id.clone(),
                    decision: decision.clone(),
                });
            }
        }
        Ok(())
    }

    /// Applies the carried records and returns the decisions whose content changed.
    fn apply_records(&mut self, number: u32, records: RecordsInput) -> Vec<DecisionId> {
        let mut changed = Vec::new();
        for input in records.decisions {
            let content = DecisionContent {
                name: input.name,
                text: input.text,
            };
            match self.records.decisions.iter_mut().find(|d| d.id == input.id) {
                Some(decision) => {
                    if decision.current() != &content {
                        self.records.revisions.push(Revision {
                            decision: input.id.clone(),
                            round: number,
                            before: decision.current().clone(),
                            after: content.clone(),
                        });
                        decision.revise(number, content);
                        changed.push(input.id);
                    }
                }
                None => {
                    let origin = match input.decided_by {
                        DecidedBy::Question(question) => Origin::Question(question),
                        DecidedBy::Fix => Origin::FixRound(number),
                    };
                    self.records.decisions.push(Decision::new(
                        input.id.clone(),
                        origin,
                        number,
                        content,
                    ));
                    changed.push(input.id);
                }
            }
        }
        if let Some(list) = records.not_building {
            self.records.not_building = list;
        }
        if let Some(list) = records.undecided {
            self.records.undecided = list;
        }
        if let Some(list) = records.delegated {
            self.records.delegated = list;
        }
        if let Some(list) = records.rejected {
            self.records.rejected = list;
        }
        changed
    }

    fn fix_with_change(&self, number: u32, fix: FixInput, changed: &[DecisionId]) -> Fix {
        let change = fix
            .decision
            .filter(|id| changed.contains(id))
            .and_then(|id| {
                let decision = self.records.decisions.iter().find(|d| d.id == id)?;
                Some(DecisionChange {
                    before: decision.content_as_of(number - 1).cloned(),
                    after: decision.current().clone(),
                    decision: id,
                })
            });
        Fix {
            text: fix.text,
            change,
        }
    }
}

fn new_question(input: QuestionInput) -> Question {
    let recommended = input
        .options
        .iter()
        .position(|option| option.recommended)
        .expect("a round is applied only after each question is checked for one recommendation");
    Question {
        answer: Answer::initial(recommended, input.class),
        id: input.id,
        text: input.text,
        class: input.class,
        why_now: input.why_now,
        premises: input.premises,
        background: input.background,
        options: input.options,
        reasks: input.reasks,
    }
}
