//! Reading directive blocks, `::name{...}` and `:::name{...}` (ADR 0012).

use khalid_core::AbsoluteUrl;

use std::time::Duration;

use crate::{BlockError, BlockKind, CalloutKind, VideoBlock};

/// Every block name, for messages.
pub(crate) const BLOCKS: [&str; 8] = [
    "bookmark", "button", "callout", "embed", "figure", "toc", "toggle", "video",
];

/// What to say after "there is no block called `name`": the nearest block
/// name when one is a typing slip away, or where the names come from.
pub(crate) fn unknown_hint(name: &str) -> String {
    BLOCKS
        .iter()
        .map(|block| (distance(name, block), block))
        .filter(|(steps, _)| *steps <= 2)
        .min()
        .map_or_else(
            || "a block's name is one word, such as callout, figure or toggle".to_owned(),
            |(_, block)| format!("the closest name is {block}"),
        )
}

/// How many letters must be added, removed or changed to turn `a` into `b`.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.chars().enumerate() {
        let mut current = vec![i.saturating_add(1)];
        for (j, right) in b.iter().enumerate() {
            let replace =
                previous.get(j).copied().unwrap_or_default() + usize::from(left != *right);
            let remove = previous
                .get(j.saturating_add(1))
                .copied()
                .unwrap_or_default()
                + 1;
            let add = current.last().copied().unwrap_or_default() + 1;
            current.push(replace.min(remove).min(add));
        }
        previous = current;
    }
    previous.last().copied().unwrap_or_default()
}

/// Blocks with no body, written on one line as `::name{...}`. Every other
/// block opens with `:::name{...}` and closes with `:::`.
const LEAVES: [&str; 5] = ["bookmark", "button", "embed", "toc", "video"];

/// How a block was written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Form {
    /// `::name{...}` on one line.
    Leaf,
    /// `:::name{...}`, a body, then `:::`.
    Container,
}

/// A `::name` or `:::name{...}` line: its run of colons and what follows.
pub(crate) fn opening(line: &str) -> Option<(&str, &str)> {
    let colons = line.len() - line.trim_start_matches(':').len();
    let (fence, rest) = line.split_at_checked(colons)?;
    (colons >= 2 && rest.starts_with(|c: char| c.is_ascii_alphabetic()))
        .then_some((fence, rest.trim_end()))
}

/// Reads a directive's name and attributes, written in `form` on body `line`,
/// into its kind.
pub(crate) fn directive(line: usize, rest: &str, form: Form) -> Result<BlockKind, BlockError> {
    let (name, braces) = rest
        .split_once('{')
        .map_or((rest, ""), |(name, braces)| (name, braces));
    let name = name.trim();
    check_form(line, name, form)?;
    let list = match braces.strip_suffix('}') {
        Some(inside) => parse_attributes(inside).ok_or(BlockError::Attributes { line })?,
        None if braces.is_empty() => Vec::new(),
        None => return Err(BlockError::Attributes { line }),
    };
    let attributes = Attributes { line, list };
    match name {
        "callout" => callout(&attributes),
        "figure" => figure(&attributes),
        "toggle" => toggle(&attributes),
        "toc" => attributes
            .only("toc", &[], "nothing")
            .map(|()| BlockKind::Toc),
        "button" => button(&attributes),
        "bookmark" => bookmark(&attributes),
        "embed" => embed(&attributes),
        "video" => video(&attributes),
        other => Err(BlockError::UnknownBlock {
            line,
            name: other.to_owned(),
        }),
    }
}

/// Refuses a known block written in the other form, naming the right one.
fn check_form(line: usize, name: &str, form: Form) -> Result<(), BlockError> {
    if !BLOCKS.contains(&name) {
        return Ok(());
    }
    let problem = match (form, LEAVES.contains(&name)) {
        (Form::Container, true) => format!("{name} has no body; write it on one line as ::{name}"),
        (Form::Leaf, false) => {
            format!("{name} has a body; open it with :::{name} and close it with :::")
        }
        _ => return Ok(()),
    };
    Err(BlockError::Form { line, problem })
}

fn callout(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("callout", &["kind"], "kind")?;
    let kind = match attributes.get("kind").as_deref() {
        Some("note") | None => CalloutKind::Note,
        Some("tip") => CalloutKind::Tip,
        Some("warning") => CalloutKind::Warning,
        Some(other) => {
            return Err(BlockError::CalloutKind {
                line: attributes.line,
                kind: other.to_owned(),
            });
        }
    };
    Ok(BlockKind::Callout { kind })
}

fn figure(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("figure", &["src", "alt"], "src and alt")?;
    let src = attributes.need("src", "a figure needs src, the address of the image")?;
    let alt = attributes.need(
        "alt",
        "a figure needs alt, the text that describes the image to people who cannot see it",
    )?;
    Ok(BlockKind::Figure { src, alt })
}

fn toggle(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("toggle", &["summary"], "summary")?;
    let summary = attributes.need(
        "summary",
        "a toggle needs summary, the line a reader clicks to open it",
    )?;
    Ok(BlockKind::Toggle { summary })
}

fn button(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("button", &["href", "label"], "href and label")?;
    let href = attributes.need("href", "a button needs href, the page it leads to")?;
    let label = attributes.need("label", "a button needs label, the words on the button")?;
    let safe = ["/", "#", "https:", "http:", "mailto:"]
        .iter()
        .any(|start| href.starts_with(start))
        || !href.contains(':');
    if !safe {
        return Err(BlockError::Link {
            line: attributes.line,
            href,
        });
    }
    Ok(BlockKind::Button { href, label })
}

fn bookmark(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only(
        "bookmark",
        &["href", "title", "description", "image"],
        "href, title, description and image",
    )?;
    let href = attributes.need("href", "a bookmark needs href, the page it links to")?;
    let image = attributes
        .get("image")
        .map(|image| attributes.address("bookmark", "image", &image))
        .transpose()?;
    Ok(BlockKind::Bookmark {
        href: attributes.address("bookmark", "href", &href)?,
        title: attributes.get("title"),
        description: attributes.get("description"),
        image,
    })
}

fn embed(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("embed", &["src", "title"], "src and title")?;
    let src = attributes.need("src", "an embed needs src, the page of the video or sound")?;
    let title = attributes.need(
        "title",
        "an embed needs title, the name a screen reader says for the player",
    )?;
    Ok(BlockKind::Embed {
        src: attributes.address("embed", "src", &src)?,
        title,
    })
}

fn video(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only(
        "video",
        &["src", "title", "poster", "description", "duration"],
        "src, title, poster, description and duration",
    )?;
    let src = attributes.need("src", "a video needs src, the video file")?;
    let title = attributes.need("title", "a video needs title, the video's name")?;
    let poster = attributes.need(
        "poster",
        "a video needs poster, the still picture shown before it plays",
    )?;
    let duration = attributes
        .get("duration")
        .map(|text| {
            clock(&text).ok_or(BlockError::Value {
                line: attributes.line,
                problem: "a video's duration is written as minutes:seconds, such as 2:05, or hours:minutes:seconds",
            })
        })
        .transpose()?;
    Ok(BlockKind::Video(VideoBlock {
        src: attributes.media("video", "src", src)?,
        title,
        poster: attributes.media("video", "poster", poster)?,
        description: attributes.get("description"),
        duration,
    }))
}

/// Reads `m:ss` or `h:mm:ss` as a duration.
fn clock(text: &str) -> Option<Duration> {
    let parts = text
        .split(':')
        .map(|part| part.parse::<u64>().ok())
        .collect::<Option<Vec<u64>>>()?;
    let (hours, minutes, seconds) = match parts.as_slice() {
        [minutes, seconds] => (0, *minutes, *seconds),
        [hours, minutes, seconds] if *minutes < 60 => (*hours, *minutes, *seconds),
        _ => return None,
    };
    let total = hours
        .checked_mul(3600)?
        .checked_add(minutes.checked_mul(60)?)?
        .checked_add(seconds)?;
    (seconds < 60).then(|| Duration::from_secs(total))
}

/// A directive's attributes, with the line they are on for messages.
struct Attributes {
    line: usize,
    list: Vec<(String, String)>,
}

impl Attributes {
    /// Refuses any attribute but `allowed`; `takes` names them for people.
    fn only(
        &self,
        block: &'static str,
        allowed: &[&str],
        takes: &'static str,
    ) -> Result<(), BlockError> {
        match self
            .list
            .iter()
            .find(|(key, _)| !allowed.contains(&key.as_str()))
        {
            Some((key, _)) => Err(BlockError::UnknownAttribute {
                line: self.line,
                block,
                attribute: key.clone(),
                takes,
            }),
            None => Ok(()),
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        self.list
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    /// `text`, the value of `attribute`, as a full web address.
    fn address(
        &self,
        block: &'static str,
        attribute: &'static str,
        text: &str,
    ) -> Result<AbsoluteUrl, BlockError> {
        AbsoluteUrl::parse(text).map_err(|source| BlockError::Address {
            line: self.line,
            block,
            attribute,
            source,
        })
    }

    /// `text`, the value of `attribute`, as a path on this site or a full
    /// web address.
    fn media(
        &self,
        block: &'static str,
        attribute: &'static str,
        text: String,
    ) -> Result<String, BlockError> {
        if text.contains(':') {
            self.address(block, attribute, &text)?;
        }
        Ok(text)
    }

    /// The value of `key`, or `problem` as the error when it is missing.
    fn need(&self, key: &str, problem: &'static str) -> Result<String, BlockError> {
        self.get(key).ok_or(BlockError::Missing {
            line: self.line,
            problem,
        })
    }
}

/// Reads `name=value name="a value"` pairs, or `None` when they cannot be read.
fn parse_attributes(text: &str) -> Option<Vec<(String, String)>> {
    let mut found = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let (key, after) = rest.split_once('=')?;
        let key = key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return None;
        }
        let (value, after) = match after.strip_prefix('"') {
            Some(quoted) => {
                let (value, after) = quoted.split_once('"')?;
                if !(after.is_empty() || after.starts_with(' ')) {
                    return None;
                }
                (value, after)
            }
            None => after.split_once(' ').unwrap_or((after, "")),
        };
        if value.contains(['"', '{', '}']) {
            return None;
        }
        found.push((key.to_owned(), value.to_owned()));
        rest = after.trim_start();
    }
    Some(found)
}
