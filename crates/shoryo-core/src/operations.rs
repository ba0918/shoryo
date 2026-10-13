//! What the person does on the screen before sending, what the agent does in reply, and the
//! events the agent waits for (`docs/spec/server.md`, "待つ", "返す", "終える";
//! `docs/spec/screen.md`, "今のラウンド").

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{AskId, DecisionId, EventId, QuestionId};
use crate::state::{
    AnswerMethod, Ask, AskState, Class, InReview, Question, Reply, Round, SentAs, Stamp, Timestamp,
    Topic,
};

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

/// Why an operation was not carried out. Serialised with its `kind`, so the screen can say it
/// in the person's language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperationRefusal {
    AnswersChanged,
    EmptyFreeText {
        questions: Vec<QuestionId>,
    },
    InvalidAnswer {
        question: QuestionId,
    },
    Explanation(crate::ExplanationError),
    NoRound,
    RoundSubmitted,
    RoundUnsent {
        round: u32,
    },
    TopicEnded,
    UnknownQuestion {
        question: QuestionId,
    },
    UnknownOption {
        option: usize,
    },
    UnknownAsk {
        ask: AskId,
    },
    UnknownDecision {
        decision: DecisionId,
    },
    AlreadyReplied {
        ask: AskId,
    },
    NotInReview {
        decision: DecisionId,
    },
    /// A review request between sending a round and the next one: the agent would not read it.
    ReviewWhileRoundSent,
    NotCurrentRound {
        round: u32,
    },
    FollowsAnotherQuestion {
        ask: AskId,
    },
    /// The round cannot be sent while these questions have no stamp.
    Unstamped {
        questions: Vec<QuestionId>,
    },
}

impl fmt::Display for OperationRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AnswersChanged => write!(
                f,
                "answers changed; confirm the latest answers before sending"
            ),
            Self::InvalidAnswer { question } => write!(f, "invalid sent answer for {question}"),
            Self::EmptyFreeText { questions } => {
                write!(f, "custom answers are empty: {questions:?}")
            }
            Self::Explanation(error) => error.fmt(f),
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
            Self::ReviewWhileRoundSent => write!(
                f,
                "the round has been sent; review requests can be sent once the next round arrives"
            ),
            Self::FollowsAnotherQuestion { ask } => {
                write!(
                    f,
                    "ask {ask} is about another question, so this cannot follow it"
                )
            }
            Self::Unstamped { questions } => {
                let list: Vec<String> = questions.iter().map(ToString::to_string).collect();
                write!(
                    f,
                    "questions {} have no stamp yet; stamp every question before sending",
                    list.join(", ")
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
                if stamped
                    && q.answer.method == AnswerMethod::Custom
                    && !q.answer.deferred
                    && q.answer.free_text.trim().is_empty()
                {
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
                self.push_event(EventKind::ClassSwapped { question, class });
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

    /// Stores the agent's reply to an ask, in whichever round the ask belongs to.
    pub fn reply(&mut self, ask: AskId, reply: Reply) -> Result<(), OperationRefusal> {
        if self.ended {
            return Err(OperationRefusal::TopicEnded);
        }
        crate::explanation_validation::validate_parts(&reply.parts, "reply", 1, &mut 0)
            .map_err(OperationRefusal::Explanation)?;
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
        self.open_question(&question)?;
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
        if self.current_round().is_some_and(|round| round.submitted) {
            return Err(OperationRefusal::ReviewWhileRoundSent);
        }
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
        if !self.is_in_review(&decision) {
            return Err(OperationRefusal::NotInReview { decision });
        }
        self.records
            .in_review
            .retain(|mark| mark.decision != decision);
        self.push_event(EventKind::ReviewStopped { decision });
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

impl Round {
    pub fn sent_answers(&self) -> Result<Vec<SentAnswer>, OperationRefusal> {
        let empty: Vec<_> = self
            .questions
            .iter()
            .filter(|q| {
                q.answer.method == AnswerMethod::Custom
                    && !q.answer.deferred
                    && q.answer.free_text.trim().is_empty()
            })
            .map(|q| q.id.clone())
            .collect();
        if !empty.is_empty() {
            return Err(OperationRefusal::EmptyFreeText { questions: empty });
        }
        let unstamped: Vec<_> = self
            .questions
            .iter()
            .filter(|q| q.answer.stamp.is_none())
            .map(|q| q.id.clone())
            .collect();
        if !unstamped.is_empty() {
            return Err(OperationRefusal::Unstamped {
                questions: unstamped,
            });
        }
        self.questions
            .iter()
            .map(|q| {
                if q.answer.selected >= q.options.len() {
                    return Err(OperationRefusal::UnknownOption {
                        option: q.answer.selected,
                    });
                }
                let custom = q.answer.method == AnswerMethod::Custom;
                if custom && q.answer.stamp != Some(Stamp::Person) {
                    return Err(OperationRefusal::InvalidAnswer {
                        question: q.id.clone(),
                    });
                }
                Ok(SentAnswer {
                    question: q.id.clone(),
                    class: q.class,
                    choice: (!q.answer.deferred && !custom).then_some(q.answer.selected),
                    free_text: (!q.answer.deferred && custom).then(|| q.answer.free_text.clone()),
                    note: if custom {
                        String::new()
                    } else {
                        q.answer.note.clone()
                    },
                    deferred: q.answer.deferred,
                    stamp: q
                        .answer
                        .stamp
                        .expect("all stamps were validated before projection"),
                })
            })
            .collect()
    }
}

impl SentAnswer {
    pub fn validate(&self) -> Result<(), OperationRefusal> {
        let valid = match (self.deferred, self.choice, &self.free_text) {
            (true, None, None) | (false, Some(_), None) => true,
            (false, None, Some(text)) => {
                !text.trim().is_empty() && self.note.is_empty() && self.stamp == Stamp::Person
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(OperationRefusal::InvalidAnswer {
                question: self.question.clone(),
            })
        }
    }
}

impl Topic {
    pub fn validate_answers(&self) -> Result<(), OperationRefusal> {
        for round in &self.rounds {
            for q in &round.questions {
                if q.answer.selected >= q.options.len() {
                    return Err(OperationRefusal::UnknownOption {
                        option: q.answer.selected,
                    });
                }
                if q.answer.method == AnswerMethod::Custom
                    && q.answer.stamp == Some(Stamp::PreApproved)
                {
                    return Err(OperationRefusal::InvalidAnswer {
                        question: q.id.clone(),
                    });
                }
            }
            if round.submitted {
                round.sent_answers()?;
            }
        }
        for event in &self.events {
            if let EventKind::Submitted { round, answers } = &event.kind {
                for answer in answers {
                    answer.validate()?;
                    let q = self
                        .rounds
                        .iter()
                        .filter(|candidate| candidate.number == *round)
                        .flat_map(|candidate| &candidate.questions)
                        .find(|q| q.id == answer.question)
                        .ok_or_else(|| OperationRefusal::InvalidAnswer {
                            question: answer.question.clone(),
                        })?;
                    if answer.choice.is_some_and(|i| i >= q.options.len()) {
                        return Err(OperationRefusal::InvalidAnswer {
                            question: answer.question.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}
