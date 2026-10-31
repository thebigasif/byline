//! The maths block: a formula on its own, checked when it is read.

use super::Attributes;
use crate::{BlockError, BlockKind, maths};

/// Reads a maths block whose formula is its `body`. A formula that cannot be
/// drawn is refused here, on its first line, so no page shows it as source.
pub(super) fn maths(attributes: &Attributes, body: &[&str]) -> Result<BlockKind, BlockError> {
    attributes.only("maths block", &["label"], "label")?;
    let formula = body.join("\n").trim().to_owned();
    if formula.is_empty() {
        return Err(BlockError::Missing {
            line: attributes.line,
            problem: "a maths block needs a formula between its ::: lines, such as E = mc^2",
        });
    }
    let first = body
        .iter()
        .position(|line| !line.trim().is_empty())
        .unwrap_or_default();
    maths::display(&formula).map_err(|problem| BlockError::Maths {
        line: attributes.line.saturating_add(1).saturating_add(first),
        problem,
    })?;
    Ok(BlockKind::Maths {
        label: attributes.get("label"),
        formula,
    })
}
