//! Reading directive blocks, `::name{...}` and `:::name{...}` (ADR 0012).

use std::time::Duration;

use khalid_core::AbsoluteUrl;

use crate::{
    BlockError, BlockKind, CalloutKind, GalleryImage, ProductBlock, REVIEW_TYPES, ReviewBlock,
    VideoBlock,
};

/// Every block name, for messages.
pub(crate) const BLOCKS: [&str; 24] = [
    "audio",
    "bookmark",
    "business",
    "button",
    "callout",
    "embed",
    "faq",
    "figure",
    "file",
    "form",
    "gallery",
    "gated",
    "header",
    "howto",
    "html",
    "maths",
    "newsletter",
    "product",
    "recipe",
    "review",
    "snippet",
    "toc",
    "toggle",
    "video",
];

mod cards;
mod forms;
mod formula;
mod nested;

pub(crate) use forms::TRAP;

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
const LEAVES: [&str; 10] = [
    "audio",
    "bookmark",
    "business",
    "button",
    "embed",
    "file",
    "newsletter",
    "snippet",
    "toc",
    "video",
];

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

/// Reads a directive's name and attributes, written in `form` on body `line`
/// with the lines of its `body`, into its kind.
pub(crate) fn directive(
    line: usize,
    rest: &str,
    form: Form,
    body: &[&str],
) -> Result<BlockKind, BlockError> {
    let (name, attributes) = read(line, rest)?;
    check_form(line, name, form)?;
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
        "audio" | "file" => sound_or_file(name, &attributes),
        "gallery" => attributes
            .only("gallery", &[], "nothing")
            .and_then(|()| gallery(line, body)),
        "faq" => nested::faq(&attributes, body),
        "howto" => nested::how_to(&attributes, body),
        "recipe" => nested::recipe(&attributes, body),
        "review" => review(&attributes, body),
        "product" => product(&attributes, body),
        "business" => cards::business(&attributes),
        "snippet" => cards::snippet(&attributes),
        "header" => cards::header(&attributes),
        "html" => cards::html(&attributes),
        "form" => forms::form(&attributes, body),
        "newsletter" | "gated" => forms::subscribe(name, &attributes),
        "maths" => formula::maths(&attributes, body),
        other => Err(BlockError::UnknownBlock {
            line,
            name: other.to_owned(),
        }),
    }
}

/// Splits a directive's `rest`, written on body `line`, into its name and
/// attributes.
fn read(line: usize, rest: &str) -> Result<(&str, Attributes), BlockError> {
    let (name, braces) = rest
        .split_once('{')
        .map_or((rest, ""), |(name, braces)| (name, braces));
    let list = match braces.strip_suffix('}') {
        Some(inside) => parse_attributes(inside).ok_or(BlockError::Attributes { line })?,
        None if braces.is_empty() => Vec::new(),
        None => return Err(BlockError::Attributes { line }),
    };
    Ok((name.trim(), Attributes { line, list }))
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

/// Reads `audio` or `file`, which both take a file and a title.
fn sound_or_file(name: &str, attributes: &Attributes) -> Result<BlockKind, BlockError> {
    let (block, needs_src, needs_title) = if name == "audio" {
        (
            "audio",
            "an audio block needs src, the sound file",
            "an audio block needs title, the words shown above the player",
        )
    } else {
        (
            "file",
            "a file block needs src, the file to download",
            "a file block needs title, the words of the download link",
        )
    };
    attributes.only(block, &["src", "title"], "src and title")?;
    let src = attributes.media(block, "src", attributes.need("src", needs_src)?)?;
    let title = attributes.need("title", needs_title)?;
    Ok(if block == "audio" {
        BlockKind::Audio { src, title }
    } else {
        BlockKind::File { src, title }
    })
}

/// Reads a gallery's body, one `![alt](src)` image per line, from the
/// `body` of the block opened on `line`.
fn gallery(line: usize, body: &[&str]) -> Result<BlockKind, BlockError> {
    let mut images = Vec::new();
    for (number, text) in (line.saturating_add(1)..).zip(body) {
        if text.trim().is_empty() {
            continue;
        }
        let image = text
            .trim()
            .strip_prefix("![")
            .and_then(|rest| rest.strip_suffix(')'))
            .and_then(|rest| rest.split_once("]("))
            .filter(|(alt, src)| !alt.trim().is_empty() && !src.trim().is_empty());
        let Some((alt, src)) = image else {
            return Err(BlockError::Value {
                line: number,
                problem: "each line of a gallery is an image with alt text, such as ![The editor](/media/editor.png)",
            });
        };
        images.push(GalleryImage {
            src: src.trim().to_owned(),
            alt: alt.trim().to_owned(),
        });
    }
    Ok(BlockKind::Gallery { images })
}

fn review(attributes: &Attributes, body: &[&str]) -> Result<BlockKind, BlockError> {
    attributes.only(
        "review",
        &["item", "type", "rating", "best", "author"],
        "item, type, rating, best and author",
    )?;
    let item = attributes.need("item", "a review needs item, the thing it reviews")?;
    let item_type = attributes.need(
        "type",
        "a review needs type, such as Product, Book or SoftwareApplication",
    )?;
    if !REVIEW_TYPES.contains(&item_type.as_str()) {
        return Err(BlockError::Value {
            line: attributes.line,
            problem: "a review's type is one search engines show, such as Product, Book, Course, SoftwareApplication or LocalBusiness",
        });
    }
    let rating = attributes.need("rating", "a review needs rating, its score")?;
    let best = attributes.get("best").unwrap_or_else(|| "5".to_owned());
    match (score(&rating), score(&best)) {
        (Some(value), Some(top)) if top > 0.0 && value <= top => {}
        _ => {
            return Err(BlockError::Value {
                line: attributes.line,
                problem: "a review's rating is a number from 0 to its best score, such as 4.5 out of 5",
            });
        }
    }
    Ok(BlockKind::Review(ReviewBlock {
        item,
        item_type,
        rating,
        best,
        author: attributes.get("author"),
        body: body.join("\n"),
    }))
}

fn product(attributes: &Attributes, body: &[&str]) -> Result<BlockKind, BlockError> {
    attributes.only(
        "product",
        &PRODUCT_ATTRIBUTES,
        "name, image, price, currency, rating, best, href and button",
    )?;
    let name = attributes.need("name", "a product needs name, what it is called")?;
    let (price, currency) = (attributes.get("price"), attributes.get("currency"));
    let money = match (&price, &currency) {
        (Some(price), Some(code)) => {
            let (units, cents) = price.split_once('.').unwrap_or((price, "00"));
            units.parse::<u64>().is_ok()
                && cents.len() == 2
                && cents.parse::<u8>().is_ok()
                && code.len() == 3
                && code.bytes().all(|b| b.is_ascii_uppercase())
        }
        (None, None) => true,
        _ => false,
    };
    let best = attributes.get("best").unwrap_or_else(|| "5".to_owned());
    let rating = attributes.get("rating");
    let rated = rating.as_deref().is_none_or(|rating| {
        matches!((score(rating), score(&best)), (Some(value), Some(top)) if top > 0.0 && value <= top)
    });
    let problem = if !money {
        "a product's price is a number such as 49.00 with a three-letter currency such as USD"
    } else if !rated {
        "a product's rating is a number from 0 to its best score, such as 4.5 out of 5"
    } else if attributes.get("href").is_some() != attributes.get("button").is_some() {
        "a product with href needs button, the words on its button"
    } else {
        ""
    };
    if !problem.is_empty() {
        return Err(BlockError::Value {
            line: attributes.line,
            problem,
        });
    }
    let address = |key: &'static str| {
        attributes
            .get(key)
            .map(|text| attributes.media("product", key, text))
            .transpose()
    };
    Ok(BlockKind::Product(ProductBlock {
        name,
        image: address("image")?,
        price,
        currency,
        rating,
        best,
        href: address("href")?,
        button: attributes.get("button"),
        description: body.join("\n"),
    }))
}

const PRODUCT_ATTRIBUTES: [&str; 8] = [
    "name", "image", "price", "currency", "rating", "best", "href", "button",
];

/// Reads a score written with digits and at most one point, such as `4.5`.
fn score(text: &str) -> Option<f64> {
    let digits = text.bytes().all(|b| b.is_ascii_digit() || b == b'.');
    digits.then(|| text.parse().ok()).flatten()
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

    /// The value of `key` read with `read`, or `problem` as the error when
    /// `read` cannot read it.
    fn parsed<T>(
        &self,
        key: &str,
        read: impl Fn(&str) -> Option<T>,
        problem: &'static str,
    ) -> Result<Option<T>, BlockError> {
        self.get(key)
            .map(|text| {
                read(&text).ok_or(BlockError::Value {
                    line: self.line,
                    problem,
                })
            })
            .transpose()
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
