use super::{EventKind, OperationRefusal, SentAnswer};
use crate::state::{AnswerMethod, Round, Stamp, Topic};

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
