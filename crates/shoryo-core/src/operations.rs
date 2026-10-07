//! What the person does on the screen before sending, what the agent does in reply, and the
//! events the agent waits for (`docs/spec/server.md`, "待つ", "返す", "終える";
//! `docs/spec/screen.md`, "今のラウンド").

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{AskId, DecisionId, EventId, QuestionId};
use crate::state::{Ask, AskState, Class, InReview, Question, Reply, Round, Topic};

/// One thing the person does on the screen.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Choose {
        question: QuestionId,
        option: usize,
    },
    Note {
        question: QuestionId,
        text: String,
    },
    Defer {
        question: QuestionId,
        deferred: bool,
    },
    Open {
        question: QuestionId,
    },
    SwapClass {
        question: QuestionId,
    },
    Ask {
        question: QuestionId,
        text: String,
        follows: Option<AskId>,
    },
    RequestReview {
        decision: DecisionId,
    },
    StopReview {
        decision: DecisionId,
    },
    /// Sends the round the person was looking at; `round` is that round's number.
    Submit {
        round: u32,
    },
}

/// Something that happened on the screen, kept until the agent acknowledges it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    Ask {
        ask: AskId,
        round: u32,
        question: QuestionId,
        text: String,
        follows: Option<AskId>,
    },
    Submitted {
        round: u32,
        answers: Vec<SentAnswer>,
    },
    ReviewRequested {
        decision: DecisionId,
    },
    ReviewStopped {
        decision: DecisionId,
    },
    ClassSwapped {
        question: QuestionId,
        class: Class,
    },
}

/// One question's answer as sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentAnswer {
    pub question: QuestionId,
    pub class: Class,
    /// The chosen option's index; `None` when the question was deferred.
    pub choice: Option<usize>,
    pub note: String,
    pub deferred: bool,
    pub sent_unseen: bool,
}

/// Why an operation was not carried out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationRefusal {
    NoRound,
    RoundSubmitted,
    RoundUnsent { round: u32 },
    TopicEnded,
    UnknownQuestion { question: QuestionId },
    UnknownOption { option: usize },
    UnknownAsk { ask: AskId },
    UnknownDecision { decision: DecisionId },
    AlreadyReplied { ask: AskId },
    NotInReview { decision: DecisionId },
    NextRoundArrived,
    NotCurrentRound { round: u32 },
    FollowsAnotherQuestion { ask: AskId },
}

impl fmt::Display for OperationRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRound => write!(f, "no round has been sent yet"),
            Self::RoundSubmitted => write!(f, "the round has already been sent"),
            Self::RoundUnsent { round } => write!(f, "round {round} has not been sent yet"),
            Self::TopicEnded => write!(f, "the topic has ended"),
            Self::UnknownQuestion { question } => {
                write!(f, "question {question} is not in the current round")
            }
            Self::UnknownOption { option } => write!(f, "there is no option {option}"),
            Self::UnknownAsk { ask } => write!(f, "there is no ask {ask}"),
            Self::UnknownDecision { decision } => write!(f, "there is no decision {decision}"),
            Self::AlreadyReplied { ask } => write!(f, "ask {ask} already has a reply"),
            Self::NotInReview { decision } => write!(f, "decision {decision} is not in review"),
            Self::NextRoundArrived => write!(
                f,
                "the next round has arrived, so the review request can no longer be stopped"
            ),
            Self::FollowsAnotherQuestion { ask } => {
                write!(
                    f,
                    "ask {ask} is about another question, so this cannot follow it"
                )
            }
            Self::NotCurrentRound { round } => write!(
                f,
                "round {round} is no longer the current round; the screen shows the current one"
            ),
        }
    }
}

impl std::error::Error for OperationRefusal {}

impl Topic {
    pub fn current_round(&self) -> Option<&Round> {
        self.rounds.last()
    }

    pub fn pending_events(&self) -> &[Event] {
        &self.events
    }

    /// Forgets the events the agent says it has received.
    pub fn acknowledge(&mut self, ids: &[EventId]) {
        self.events.retain(|event| !ids.contains(&event.id));
    }

    /// Whether the agent may wait for events now.
    pub fn check_wait(&self) -> Result<(), OperationRefusal> {
        if self.ended {
            Err(OperationRefusal::TopicEnded)
        } else {
            Ok(())
        }
    }

    pub fn is_in_review(&self, decision: &DecisionId) -> bool {
        self.records
            .in_review
            .iter()
            .any(|mark| &mark.decision == decision)
    }

    /// まだ開いてない印: a human question in the unsent current round that was neither opened
    /// nor touched.
    pub fn shows_unopened_mark(&self, question: &QuestionId) -> bool {
        self.current_round()
            .filter(|round| !round.submitted)
            .and_then(|round| round.questions.iter().find(|q| &q.id == question))
            .is_some_and(is_unopened)
    }

    /// Carries out one of the person's operations; nothing changes when it is refused.
    /// Opening a card is reading, which stays possible after sending and after the end, so
    /// it is never refused for those; it only has nothing left to record.
    pub fn apply(&mut self, operation: Operation) -> Result<(), OperationRefusal> {
        if self.ended && !matches!(operation, Operation::Open { .. }) {
            return Err(OperationRefusal::TopicEnded);
        }
        match operation {
            Operation::Choose { question, option } => {
                let q = self.open_question(&question)?;
                if option >= q.options.len() {
                    return Err(OperationRefusal::UnknownOption { option });
                }
                q.answer.selected = option;
                q.answer.touched = true;
            }
            Operation::Note { question, text } => {
                let q = self.open_question(&question)?;
                q.answer.note = text;
                q.answer.touched = true;
            }
            Operation::Defer { question, deferred } => {
                let q = self.open_question(&question)?;
                q.answer.deferred = deferred;
                q.answer.touched = true;
            }
            Operation::Open { question } => {
                if self.ended || self.current_round().is_some_and(|round| round.submitted) {
                    return Ok(());
                }
                self.open_question(&question)?.answer.opened = true;
            }
            Operation::SwapClass { question } => {
                let q = self.open_question(&question)?;
                q.class = match q.class {
                    Class::Human => Class::Provisional,
                    Class::Provisional => Class::Human,
                };
                let class = q.class;
                self.push_event(EventKind::ClassSwapped { question, class });
            }
            Operation::Ask {
                question,
                text,
                follows,
            } => self.ask(question, text, follows)?,
            Operation::RequestReview { decision } => self.request_review(decision)?,
            Operation::StopReview { decision } => self.stop_review(decision)?,
            Operation::Submit { round } => self.submit(round)?,
        }
        Ok(())
    }

    /// Stores the agent's reply to an ask, in whichever round the ask belongs to.
    pub fn reply(&mut self, ask: AskId, reply: Reply) -> Result<(), OperationRefusal> {
        if self.ended {
            return Err(OperationRefusal::TopicEnded);
        }
        let found = self
            .rounds
            .iter_mut()
            .flat_map(|round| round.asks.iter_mut())
            .find(|candidate| candidate.id == ask)
            .ok_or(OperationRefusal::UnknownAsk { ask })?;
        match found.state {
            AskState::Waiting => {
                found.state = AskState::Replied(reply);
                Ok(())
            }
            AskState::Replied(_) | AskState::NoReply => {
                Err(OperationRefusal::AlreadyReplied { ask })
            }
        }
    }

    /// Ends the topic; asks still waiting get "no reply".
    pub fn end(&mut self) -> Result<(), OperationRefusal> {
        if let Some(round) = self.current_round()
            && !round.submitted
        {
            return Err(OperationRefusal::RoundUnsent {
                round: round.number,
            });
        }
        for ask in self
            .rounds
            .iter_mut()
            .flat_map(|round| round.asks.iter_mut())
        {
            if ask.state == AskState::Waiting {
                ask.state = AskState::NoReply;
            }
        }
        self.ended = true;
        Ok(())
    }

    /// The question of the current round, while that round is still open for answers.
    fn open_question(&mut self, id: &QuestionId) -> Result<&mut Question, OperationRefusal> {
        let round = self.open_round()?;
        round
            .questions
            .iter_mut()
            .find(|question| &question.id == id)
            .ok_or_else(|| OperationRefusal::UnknownQuestion {
                question: id.clone(),
            })
    }

    fn open_round(&mut self) -> Result<&mut Round, OperationRefusal> {
        let round = self.rounds.last_mut().ok_or(OperationRefusal::NoRound)?;
        if round.submitted {
            return Err(OperationRefusal::RoundSubmitted);
        }
        Ok(round)
    }

    fn ask(
        &mut self,
        question: QuestionId,
        text: String,
        follows: Option<AskId>,
    ) -> Result<(), OperationRefusal> {
        let round = self.open_round()?;
        if let Some(ask) = follows {
            let followed = round
                .asks
                .iter()
                .find(|candidate| candidate.id == ask)
                .ok_or(OperationRefusal::UnknownAsk { ask })?;
            if followed.question != question {
                return Err(OperationRefusal::FollowsAnotherQuestion { ask });
            }
        }
        let number = round.number;
        self.open_question(&question)?.answer.touched = true;
        let id = self.allocate_ask_id();
        self.open_round()?.asks.push(Ask {
            id,
            question: question.clone(),
            text: text.clone(),
            follows,
            state: AskState::Waiting,
        });
        self.push_event(EventKind::Ask {
            ask: id,
            round: number,
            question,
            text,
            follows,
        });
        Ok(())
    }

    fn request_review(&mut self, decision: DecisionId) -> Result<(), OperationRefusal> {
        if !self.records.decisions.iter().any(|d| d.id == decision) {
            return Err(OperationRefusal::UnknownDecision { decision });
        }
        if self.is_in_review(&decision) {
            return Ok(());
        }
        let since_round = self.current_round().map_or(0, |round| round.number);
        self.records.in_review.push(InReview {
            decision: decision.clone(),
            since_round,
        });
        self.push_event(EventKind::ReviewRequested { decision });
        Ok(())
    }

    fn stop_review(&mut self, decision: DecisionId) -> Result<(), OperationRefusal> {
        let current = self.current_round().map_or(0, |round| round.number);
        let mark = self
            .records
            .in_review
            .iter()
            .find(|mark| mark.decision == decision)
            .ok_or_else(|| OperationRefusal::NotInReview {
                decision: decision.clone(),
            })?;
        if mark.since_round != current {
            return Err(OperationRefusal::NextRoundArrived);
        }
        self.records
            .in_review
            .retain(|mark| mark.decision != decision);
        self.push_event(EventKind::ReviewStopped { decision });
        Ok(())
    }

    fn submit(&mut self, made_on: u32) -> Result<(), OperationRefusal> {
        if self
            .current_round()
            .is_some_and(|round| round.number != made_on)
        {
            return Err(OperationRefusal::NotCurrentRound { round: made_on });
        }
        let round = self.open_round()?;
        round.submitted = true;
        for question in &mut round.questions {
            question.answer.sent_unseen = is_unopened(question);
        }
        let number = round.number;
        let answers = round
            .questions
            .iter()
            .map(|question| SentAnswer {
                question: question.id.clone(),
                class: question.class,
                choice: (!question.answer.deferred).then_some(question.answer.selected),
                note: question.answer.note.clone(),
                deferred: question.answer.deferred,
                sent_unseen: question.answer.sent_unseen,
            })
            .collect();
        self.push_event(EventKind::Submitted {
            round: number,
            answers,
        });
        Ok(())
    }

    fn push_event(&mut self, kind: EventKind) {
        let id = self.allocate_event_id();
        self.events.push(Event { id, kind });
    }
}

fn is_unopened(question: &Question) -> bool {
    question.class == Class::Human && !question.answer.opened && !question.answer.touched
}
