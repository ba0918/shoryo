use super::{EventKind, OperationRefusal};
use crate::ids::{AskId, QuestionId};
use crate::state::{Ask, AskState, Reply, Topic};

impl Topic {
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

    pub(super) fn ask(
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
        let (id, event) = self.allocate_ask_event_ids()?;
        self.open_round()?.asks.push(Ask {
            id,
            question: question.clone(),
            text: text.clone(),
            follows,
            state: AskState::Waiting,
        });
        self.push_event(
            event,
            EventKind::Ask {
                ask: id,
                round: number,
                question,
                text,
                follows,
            },
        );
        Ok(())
    }
}
