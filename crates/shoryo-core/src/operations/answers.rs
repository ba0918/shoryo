use super::{EventKind, OperationRefusal, SentAnswer};
use crate::state::{Answer, AnswerMethod, Question, Round, Stamp, Topic};

impl Round {
    pub fn sent_answers(&self) -> Result<Vec<SentAnswer>, OperationRefusal> {
        self.check_sent_answers()?;
        self.questions.iter().map(Question::sent_answer).collect()
    }

    fn check_sent_answers(&self) -> Result<(), OperationRefusal> {
        let empty: Vec<_> = self
            .questions
            .iter()
            .filter(|q| q.answer.empty_custom_body())
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
        for question in &self.questions {
            question.check_answer()?;
        }
        Ok(())
    }
}

impl Answer {
    pub(super) fn empty_custom_body(&self) -> bool {
        self.method == AnswerMethod::Custom && !self.deferred && self.free_text.trim().is_empty()
    }
}

impl Question {
    pub(crate) fn sent_answer(&self) -> Result<SentAnswer, OperationRefusal> {
        if self.answer.empty_custom_body() {
            return Err(OperationRefusal::EmptyFreeText {
                questions: vec![self.id.clone()],
            });
        }
        let stamp = self
            .answer
            .stamp
            .ok_or_else(|| OperationRefusal::Unstamped {
                questions: vec![self.id.clone()],
            })?;
        self.check_answer()?;
        let custom = self.answer.method == AnswerMethod::Custom;
        Ok(SentAnswer {
            question: self.id.clone(),
            class: self.class,
            choice: (!self.answer.deferred && !custom).then_some(self.answer.selected),
            free_text: (!self.answer.deferred && custom).then(|| self.answer.free_text.clone()),
            note: if custom {
                String::new()
            } else {
                self.answer.note.clone()
            },
            deferred: self.answer.deferred,
            stamp,
        })
    }

    fn check_answer(&self) -> Result<(), OperationRefusal> {
        if self.answer.selected >= self.options.len() {
            return Err(OperationRefusal::UnknownOption {
                option: self.answer.selected,
            });
        }
        if self.answer.method == AnswerMethod::Custom
            && self.answer.stamp == Some(Stamp::PreApproved)
        {
            return Err(OperationRefusal::InvalidAnswer {
                question: self.id.clone(),
            });
        }
        Ok(())
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
                q.check_answer()?;
            }
            if round.submitted {
                round.check_sent_answers()?;
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
