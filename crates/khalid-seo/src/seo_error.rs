//! The one error type of this crate (BL-ERR-1).

use khalid_core::ParseUrlError;

/// Something in the SEO input that cannot become valid output.
///
/// The binary turns each variant into a message with what, why and fix
/// (BL-ERR-4). The text here names the problem only.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SeoError {
    /// A path is not a clean absolute path such as `/blog/why-links-matter/`.
    #[error("the path {path:?} {problem}")]
    Path {
        /// The path, as given.
        path: String,
        /// What is wrong with it.
        problem: &'static str,
    },
    /// The address built for a page is not a valid absolute URL.
    #[error("the address built for {path:?} is not a valid URL")]
    Url {
        /// The page path the address was built for.
        path: String,
        /// Why the address was refused.
        #[source]
        source: ParseUrlError,
    },
}
