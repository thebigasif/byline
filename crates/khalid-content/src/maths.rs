//! Maths: `$...$` inline and `$$...$$` on its own lines, drawn as `MathML` at
//! build, which browsers show with scripts off (ADR "maths with math-core").

use std::sync::OnceLock;

use comrak::nodes::{AstNode, NodeMath, NodeValue};
use math_core::{LatexToMathML, MathCoreConfig, MathDisplay};

/// The `MathML` for a formula, as a block for `$$`, or why it cannot be
/// drawn, in a phrase.
pub(crate) fn mathml(math: &NodeMath) -> Result<String, String> {
    let Some(converter) = converter() else {
        return Err("the maths reader could not start".to_owned());
    };
    let style = if math.display_math {
        MathDisplay::Block
    } else {
        MathDisplay::Inline
    };
    converter
        .convert_with_local_state(math.literal.trim(), style)
        .map(|converted| converted.mathml)
        .map_err(|error| error.label().to_owned())
}

/// Turns every formula under `root` into its `MathML`. A formula that cannot
/// be drawn keeps its source, shown as escaped text; the content check
/// refuses such a site before it is built.
pub(crate) fn draw<'a>(root: &'a AstNode<'a>) {
    for node in root.descendants() {
        let mut data = node.data.borrow_mut();
        let NodeValue::Math(math) = &data.value else {
            continue;
        };
        if let Ok(drawn) = mathml(math) {
            data.value = NodeValue::Raw(drawn);
        }
    }
}

/// The converter, built once: every formula of every post shares it
/// (RULES.md BL-PERF-2). Unknown commands are errors, so a typo is caught
/// rather than drawn in red.
fn converter() -> Option<&'static LatexToMathML> {
    static CONVERTER: OnceLock<Option<LatexToMathML>> = OnceLock::new();
    CONVERTER
        .get_or_init(|| LatexToMathML::new(MathCoreConfig::default(), Vec::new()).ok())
        .as_ref()
}
