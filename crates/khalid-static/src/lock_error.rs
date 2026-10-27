//! What can go wrong while reading or locking a page.

/// Why a page could not be read or locked.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LockError {
    /// A form starts but never ends, so the page cannot be locked safely.
    #[error("the form at byte {offset} has no closing </form> tag")]
    Unclosed {
        /// Where the form's opening tag starts in the page.
        offset: usize,
    },
}
