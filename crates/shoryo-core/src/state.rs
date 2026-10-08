//! The topic's state: everything `docs/spec/server.md` ("状態データ") lists, and its JSON form.

use serde::{Deserialize, Serialize};

use crate::ids::{AskId, DecisionId, EventId, QuestionId};
use crate::operations::Event;

/// One topic: the unit shoryo starts for and keeps data for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Topic {
    pub title: String,
    /// The topic in the person's own words.
    pub original_request: String,
    pub ended: bool,
    pub rounds: Vec<Round>,
    pub records: Records,
    /// The latest finished picture, as diagram text.
    pub finished_picture: Option<String>,
    /// Events the agent has not acknowledged yet.
    pub(crate) events: Vec<Event>,
    next_ask: u64,
    next_event: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Round {
    pub number: u32,
    /// The subject, shown as the map column's heading.
    pub subject: String,
    pub review: bool,
    pub fixes: Vec<Fix>,
    pub questions: Vec<Question>,
    pub review_conclusions: Vec<ReviewConclusion>,
    /// Points the agent confirmed are still right after a premise changed.
    pub confirmed: Vec<NodeRef>,
    pub asks: Vec<Ask>,
    pub submitted: bool,
    /// The finished picture as it stood once this round was applied.
    pub finished_picture: Option<String>,
    /// The records other than decisions as they stood once this round was applied.
    pub records: RoundRecords,
    /// When the person sent the round.
    pub sent_at: Option<Timestamp>,
    /// For a sent result round: whether the person proceeded or asked for reviews.
    pub sent_as: Option<SentAs>,
}

/// The records a round showed besides decisions, kept so a past result can be drawn as it
/// was sent. Decisions need no copy: their history gives the content as of any round.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundRecords {
    pub not_building: Vec<String>,
    pub undecided: Vec<Undecided>,
    pub delegated: Vec<Delegated>,
    pub rejected: Vec<Rejected>,
}

/// How the person sent a result round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SentAs {
    /// この結果で進める: sent with no decision in review.
    Proceeded,
    /// 見直しを頼む: sent while some decision was in review.
    ReviewRequested,
}

/// 直したこと: a fix the agent derived from the records in a review round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fix {
    pub text: String,
    pub change: Option<DecisionChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionChange {
    pub decision: DecisionId,
    pub before: Option<DecisionContent>,
    pub after: DecisionContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// 人が決める問い
    Human,
    /// 仮決め
    Provisional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub id: QuestionId,
    pub text: String,
    /// The current class: the person's choice once they swapped it.
    pub class: Class,
    pub why_now: String,
    pub premises: Vec<DecisionId>,
    /// 前提知識: terms and background needed to read the question.
    pub background: String,
    pub options: Vec<Choice>,
    /// The deferred question this one asks again.
    pub reasks: Option<QuestionId>,
    pub answer: Answer,
}

// The agent writes it inside a round, where a misspelt field must be refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub text: String,
    pub description: String,
    pub recommended: bool,
    /// この答えだと: what follows from choosing this option.
    pub consequence: String,
}

/// The answer as it stands: the pre-submit state until the round is sent, the answer after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    /// Index into the question's options.
    pub selected: usize,
    pub note: String,
    pub deferred: bool,
    /// 判子: whether the answer (deferring included) has been confirmed, and by whom.
    pub stamp: Option<Stamp>,
    /// When the person last pressed the stamp; only the person's stamp has one.
    pub stamped_at: Option<Timestamp>,
}

/// A moment as the server's clock read it when it accepted something, in UTC. The domain
/// never reads a clock: the server passes the time in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(String);

impl Timestamp {
    /// `text` is an RFC 3339 time in UTC, as the server writes it.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

/// Who put the 判子 on an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stamp {
    /// The person pressed it.
    Person,
    /// 代決: the LLM pressed it for a provisional question, and the person left it.
    PreApproved,
}

impl Stamp {
    /// The stamp a question of `class` starts with: none for a human question, 代決 for a
    /// provisional one.
    pub fn initial(class: Class) -> Option<Self> {
        match class {
            Class::Human => None,
            Class::Provisional => Some(Self::PreApproved),
        }
    }
}

// The agent writes it inside a round, where a misspelt field must be refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewConclusion {
    pub decision: DecisionId,
    pub outcome: ReviewOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewOutcome {
    /// そのままでいい
    Unchanged,
    /// The content was revised in this round.
    Changed,
}

/// A point on the map: a decision or a question.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRef {
    Question(QuestionId),
    Decision(DecisionId),
}

/// 聞き返し: a question the person asked about one question before sending.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ask {
    pub id: AskId,
    pub question: QuestionId,
    pub text: String,
    /// The ask whose reply this one follows up on.
    pub follows: Option<AskId>,
    pub state: AskState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AskState {
    Waiting,
    Replied(Reply),
    /// The topic ended before a reply came.
    NoReply,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reply {
    pub text: String,
    /// A diagram as diagram text.
    pub diagram: Option<String>,
}

/// The six records and the decisions in review.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Records {
    pub decisions: Vec<Decision>,
    /// 作らないもの
    pub not_building: Vec<String>,
    /// 未決
    pub undecided: Vec<Undecided>,
    /// 任せたこと
    pub delegated: Vec<Delegated>,
    /// 見送った案
    pub rejected: Vec<Rejected>,
    /// 改めたこと
    pub revisions: Vec<Revision>,
    /// 見直し中のもの
    pub in_review: Vec<InReview>,
}

// The agent writes it inside a round, where a misspelt field must be refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Undecided {
    pub text: String,
    /// Who decides it.
    pub decider: String,
}

// The agent writes it inside a round, where a misspelt field must be refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delegated {
    pub text: String,
    pub reason: String,
}

// The agent writes it inside a round, where a misspelt field must be refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rejected {
    pub text: String,
    pub reason: String,
    /// The question in which it was rejected.
    pub question: QuestionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision {
    pub decision: DecisionId,
    pub round: u32,
    pub before: DecisionContent,
    pub after: DecisionContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InReview {
    pub decision: DecisionId,
    /// The round that was current when the person asked for the review.
    pub since_round: u32,
}

/// 決まったこと, with every content it has had.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub id: DecisionId,
    pub origin: Origin,
    /// Each content with the round it took effect in, oldest first; never empty.
    history: Vec<Version>,
}

/// Where a decision was decided.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Question(QuestionId),
    /// A fix in the review round with this number.
    FixRound(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionContent {
    /// The short name used in chains and on the map.
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Version {
    round: u32,
    content: DecisionContent,
}

/// The stored state could not be read.
#[derive(Debug)]
pub struct LoadError(serde_json::Error);

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the topic's state is not valid: {}", self.0)
    }
}

impl std::error::Error for LoadError {}

impl Topic {
    pub fn new(title: impl Into<String>, original_request: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            original_request: original_request.into(),
            ended: false,
            rounds: Vec::new(),
            records: Records::default(),
            finished_picture: None,
            events: Vec::new(),
            next_ask: 1,
            next_event: 1,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .expect("the state holds only strings, numbers, booleans and lists of them")
    }

    pub fn from_json(json: &str) -> Result<Self, LoadError> {
        serde_json::from_str(json).map_err(LoadError)
    }

    pub fn allocate_ask_id(&mut self) -> AskId {
        let id = AskId(self.next_ask);
        self.next_ask += 1;
        id
    }

    pub fn allocate_event_id(&mut self) -> EventId {
        let id = EventId(self.next_event);
        self.next_event += 1;
        id
    }
}

impl Answer {
    /// The initial answer of a question of `class`: the recommended option selected, with
    /// the stamp that class starts with.
    pub fn initial(selected: usize, class: Class) -> Self {
        Self {
            selected,
            note: String::new(),
            deferred: false,
            stamp: Stamp::initial(class),
            stamped_at: None,
        }
    }

    /// Puts `stamp` on the answer; the time is kept only for the person's stamp.
    pub(crate) fn set_stamp(&mut self, stamp: Option<Stamp>, at: Timestamp) {
        self.stamped_at = match stamp {
            Some(Stamp::Person) => Some(at),
            Some(Stamp::PreApproved) | None => None,
        };
        self.stamp = stamp;
    }
}

impl Decision {
    pub fn new(id: DecisionId, origin: Origin, round: u32, content: DecisionContent) -> Self {
        Self {
            id,
            origin,
            history: vec![Version { round, content }],
        }
    }

    /// Records a new content that takes effect in `round`.
    pub fn revise(&mut self, round: u32, content: DecisionContent) {
        self.history.push(Version { round, content });
    }

    pub fn current(&self) -> &DecisionContent {
        &self
            .history
            .last()
            .expect("a decision is created with one content and never loses it")
            .content
    }

    /// The content the decision had as of `round`, or `None` before it was decided.
    pub fn content_as_of(&self, round: u32) -> Option<&DecisionContent> {
        self.history
            .iter()
            .rev()
            .find(|version| version.round <= round)
            .map(|version| &version.content)
    }

    /// Whether the content changed after it was first decided.
    pub fn was_revised(&self) -> bool {
        self.history.len() > 1
    }

    /// The round the decision was first decided in.
    pub fn decided_in(&self) -> u32 {
        self.history.first().map_or(0, |version| version.round)
    }
}
