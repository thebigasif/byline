//! Typed data from blocks that search engines read as schema markup.

use std::time::Duration;

/// A block whose content becomes schema markup, in the order of the body,
/// for the schema builders in `khalid-seo` (task 1.14).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StructuredBlock {
    /// A video block, which becomes a `VideoObject`.
    Video(VideoBlock),
    /// An FAQ block, which becomes an `FAQPage`. Answers are plain text.
    Faq(Vec<Question>),
    /// A how-to block, which becomes a `HowTo`. Steps are plain text.
    HowTo(HowToBlock),
}

/// One question of an FAQ and its answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// The question.
    pub text: String,
    /// The answer: markdown in a block, plain text in structured data.
    pub answer: String,
}

/// A how-to: steps that help a reader do one thing, from `::::howto{...}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HowToBlock {
    /// What it helps a reader do.
    pub name: String,
    /// How long it takes in all.
    pub time: Option<Duration>,
    /// The steps, in order.
    pub steps: Vec<Step>,
}

/// One step of a how-to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// A few words for the step.
    pub name: String,
    /// What to do: markdown in a block, plain text in structured data.
    pub text: String,
}

/// A video file played on the page, from `::video{...}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoBlock {
    /// The video file: a path on this site or a full address.
    pub src: String,
    /// The video's title.
    pub title: String,
    /// The still picture shown before it plays: a path or a full address.
    pub poster: String,
    /// What the video shows, in a line.
    pub description: Option<String>,
    /// How long it runs.
    pub duration: Option<Duration>,
}
