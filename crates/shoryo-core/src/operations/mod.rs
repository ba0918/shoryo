//! What the person does on the screen before sending, what the agent does in reply, and the
//! events the agent waits for (`docs/spec/server.md`, "待つ", "返す", "終える";
//! `docs/spec/screen.md`, "今のラウンド").

mod answers;
mod interactions;
mod refusal;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::ids::{AskId, DecisionId, EventId, QuestionId};
use crate::state::{
    AnswerMethod, Class, InReview, Question, Round, SentAs, Stamp, Timestamp, Topic,
};

pub use refusal::OperationRefusal;

/// One thing the person does on the screen.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    AnswerMethod {
        question: QuestionId,
        method: AnswerMethod,
    },
    FreeText {
        question: QuestionId,
        text: String,
    },
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
    /// 判子: `stamped` presses the person's stamp; clearing it is how a 代決 is sent back.
    Stamp {
        question: QuestionId,
        stamped: bool,
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
        #[serde(default)]
        expected_answers: Option<Vec<SentAnswer>>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub free_text: Option<String>,
    pub note: String,
    pub deferred: bool,
    /// Who stamped the answer; every sent answer has a stamp.
    pub stamp: Stamp,
}

impl Topic {
    pub fn current_round(&self) -> Option<&Round> {
        self.rounds.last()
    }

    pub fn pending_events(&self) -> &[Event] {
        &self.events
    }

    /// Forgets the events the agent says it has received.
    pub fn acknowledge(&mut self, ids: &[EventId]) {
        match ids {
            [] => {}
            [id] => self.events.retain(|event| event.id != *id),
            _ => {
                let acknowledged: HashSet<_> = ids.iter().copied().collect();
                self.events
                    .retain(|event| !acknowledged.contains(&event.id));
            }
        }
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

    /// The questions of the unsent current round that have no stamp, in their order.
    pub fn unstamped(&self) -> Vec<QuestionId> {
        self.current_round()
            .filter(|round| !round.submitted)
            .into_iter()
            .flat_map(|round| &round.questions)
            .filter(|question| question.answer.stamp.is_none())
            .map(|question| question.id.clone())
            .collect()
    }

    /// Carries out one of the person's operations, accepted at `now`; nothing changes when it
    /// is refused.
    pub fn apply(&mut self, operation: Operation, now: Timestamp) -> Result<(), OperationRefusal> {
        if self.ended {
            return Err(OperationRefusal::TopicEnded);
        }
        match operation {
            Operation::AnswerMethod { question, method } => {
                let q = self.open_question(&question)?;
                if q.answer.method != method {
                    q.answer.method = method;
                    q.answer.set_stamp(None, now);
                }
            }
            Operation::FreeText { question, text } => {
                let q = self.open_question(&question)?;
                if q.answer.free_text != text {
                    q.answer.free_text = text;
                    q.answer.set_stamp(None, now);
                }
            }
            Operation::Choose { question, option } => {
                let q = self.open_question(&question)?;
                if option >= q.options.len() {
                    return Err(OperationRefusal::UnknownOption { option });
                }
                // A changed answer needs stamping again; the same answer keeps its stamp.
                if q.answer.selected != option || q.answer.method != AnswerMethod::Choice {
                    q.answer.selected = option;
                    q.answer.method = AnswerMethod::Choice;
                    q.answer.set_stamp(None, now);
                }
            }
            Operation::Note { question, text } => {
                self.open_question(&question)?.answer.note = text;
            }
            Operation::Defer { question, deferred } => {
                let q = self.open_question(&question)?;
                if q.answer.deferred != deferred {
                    q.answer.deferred = deferred;
                    q.answer.set_stamp(None, now);
                }
            }
            Operation::Stamp { question, stamped } => {
                let q = self.open_question(&question)?;
                if stamped && q.answer.empty_custom_body() {
                    return Err(OperationRefusal::EmptyFreeText {
                        questions: vec![question],
                    });
                }
                match (stamped, q.answer.stamp) {
                    (false, _) => q.answer.set_stamp(None, now),
                    // Pressing a stamp already there keeps it and the time it was pressed.
                    (true, Some(_)) => {}
                    (true, None) => q.answer.set_stamp(Some(Stamp::Person), now),
                }
            }
            Operation::SwapClass { question } => {
                // Checked before allocating so an unknown question uses up no event ID, and
                // looked up again because allocating borrows the whole topic.
                self.open_question(&question)?;
                let event = self.allocate_event_id()?;
                let q = self.open_question(&question)?;
                q.class = match q.class {
                    Class::Human => Class::Provisional,
                    Class::Provisional => Class::Human,
                };
                let stamp = match q.answer.method {
                    AnswerMethod::Choice => Stamp::initial(q.class),
                    AnswerMethod::Custom => None,
                };
                q.answer.set_stamp(stamp, now);
                let class = q.class;
                self.push_event(event, EventKind::ClassSwapped { question, class });
            }
            Operation::Ask {
                question,
                text,
                follows,
            } => self.ask(question, text, follows)?,
            Operation::RequestReview { decision } => self.request_review(decision)?,
            Operation::StopReview { decision } => self.stop_review(decision)?,
            Operation::Submit {
                round,
                expected_answers,
            } => self.submit(round, expected_answers.as_deref(), now)?,
        }
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

    fn request_review(&mut self, decision: DecisionId) -> Result<(), OperationRefusal> {
        if self.current_round().is_some_and(|round| round.submitted) {
            return Err(OperationRefusal::ReviewWhileRoundSent);
        }
        if self.find_decision(&decision).is_none() {
            return Err(OperationRefusal::UnknownDecision { decision });
        }
        if self.is_in_review(&decision) {
            return Ok(());
        }
        let since_round = self.current_round().map_or(0, |round| round.number);
        let event = self.allocate_event_id()?;
        self.records.in_review.push(InReview {
            decision: decision.clone(),
            since_round,
        });
        self.push_event(event, EventKind::ReviewRequested { decision });
        Ok(())
    }

    fn stop_review(&mut self, decision: DecisionId) -> Result<(), OperationRefusal> {
        if !self.is_in_review(&decision) {
            return Err(OperationRefusal::NotInReview { decision });
        }
        let event = self.allocate_event_id()?;
        self.records
            .in_review
            .retain(|mark| mark.decision != decision);
        self.push_event(event, EventKind::ReviewStopped { decision });
        Ok(())
    }

    fn submit(
        &mut self,
        made_on: u32,
        expected: Option<&[SentAnswer]>,
        now: Timestamp,
    ) -> Result<(), OperationRefusal> {
        if self
            .current_round()
            .is_some_and(|round| round.number != made_on)
        {
            return Err(OperationRefusal::NotCurrentRound { round: made_on });
        }
        self.open_round()?;
        let answers = self
            .current_round()
            .ok_or(OperationRefusal::NoRound)?
            .sent_answers()?;
        if let Some(expected) = expected {
            for answer in expected {
                answer.validate()?;
            }
            if expected != answers {
                return Err(OperationRefusal::AnswersChanged);
            }
        }
        let event = self.allocate_event_id()?;
        let sent_as = if self.records.in_review.is_empty() {
            SentAs::Proceeded
        } else {
            SentAs::ReviewRequested
        };
        let round = self.open_round()?;
        round.submitted = true;
        round.sent_at = Some(now);
        round.sent_as = round.questions.is_empty().then_some(sent_as);
        let number = round.number;
        self.push_event(
            event,
            EventKind::Submitted {
                round: number,
                answers,
            },
        );
        Ok(())
    }

    fn push_event(&mut self, id: EventId, kind: EventKind) {
        self.events.push(Event { id, kind });
    }
}
