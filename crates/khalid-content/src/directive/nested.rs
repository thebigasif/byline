//! Blocks that hold other blocks, written with more colons on the outer one:
//! `::::faq` around `:::question{...}` (ADR 0012).

use super::{Attributes, opening, read};
use crate::{BlockError, BlockKind, Question};

/// One inner block: its attributes and its body's markdown.
struct Child {
    attributes: Attributes,
    text: String,
}

/// Reads an FAQ opened on `line`: one `:::question{text="..."}` per
/// question, the answer as its body.
pub(super) fn faq(attributes: &Attributes, body: &[&str]) -> Result<BlockKind, BlockError> {
    attributes.only("faq", &[], "nothing")?;
    let questions = children(attributes.line, body, "question", FAQ_SHAPE)?
        .into_iter()
        .map(|child| {
            child.attributes.only("question", &["text"], "text")?;
            Ok(Question {
                text: child
                    .attributes
                    .need("text", "a question needs text, the question")?,
                answer: child.text,
            })
        })
        .collect::<Result<Vec<_>, BlockError>>()?;
    if questions.is_empty() {
        return Err(BlockError::Value {
            line: attributes.line,
            problem: "an FAQ needs at least one question, written :::question{text=\"...\"}, its answer, then :::",
        });
    }
    Ok(BlockKind::Faq { questions })
}

const FAQ_SHAPE: &str =
    "an FAQ holds only questions, each written :::question{text=\"...\"}, its answer, then :::";

/// Reads the `:::child{...}` blocks in the `body` of a block opened on
/// `line`. Blank lines between them are fine; any other line is refused with
/// `shape`, which says how the block is written.
fn children(
    line: usize,
    body: &[&str],
    child: &str,
    shape: &'static str,
) -> Result<Vec<Child>, BlockError> {
    let mut found = Vec::new();
    let mut index = 0;
    while let Some(text) = body.get(index) {
        let number = line.saturating_add(1).saturating_add(index);
        if text.trim().is_empty() {
            index = index.saturating_add(1);
            continue;
        }
        let rest = match opening(text) {
            Some((":::", rest)) if rest.split('{').next() == Some(child) => rest,
            _ => {
                return Err(BlockError::Value {
                    line: number,
                    problem: shape,
                });
            }
        };
        let Some(close) = body
            .iter()
            .skip(index.saturating_add(1))
            .position(|line| line.trim_end() == ":::")
            .map(|offset| index.saturating_add(1).saturating_add(offset))
        else {
            return Err(BlockError::Unclosed {
                line: number,
                name: child.to_owned(),
            });
        };
        let (_, attributes) = read(number, rest)?;
        let text = body
            .get(index.saturating_add(1)..close)
            .unwrap_or_default()
            .join("\n");
        found.push(Child { attributes, text });
        index = close.saturating_add(1);
    }
    Ok(found)
}
