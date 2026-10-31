//! Why the server could not hold its pages or keep serving.

use crate::PagePath;

/// Why the server could not hold its pages or keep serving.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServerError {
    /// Two pages claim one address.
    #[error("two pages claim the address {path}")]
    DuplicatePage {
        /// The address both claim.
        path: PagePath,
    },
    /// A frame origin for the Content Security Policy is not `https://`
    /// and a host.
    #[error("the frame origin {origin:?} is not an https origin")]
    FrameOrigin {
        /// The origin as given.
        origin: String,
    },
    /// A script's integrity value for the Content Security Policy is not
    /// a SHA-2 hash in base64.
    #[error("the script hash {integrity:?} is not a sha256, sha384 or sha512 value")]
    ScriptHash {
        /// The value as given.
        integrity: String,
    },
    /// The site's renderer failed, so the pages being served stay as they
    /// were.
    #[error("the site could not be rendered")]
    Render {
        /// What the renderer reported.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// The thread that renders the site stopped before it finished.
    #[error("the rendering thread stopped before it finished")]
    Worker {
        /// What the runtime reported.
        #[source]
        source: tokio::task::JoinError,
    },
    /// A page kept only as its Brotli copy could not be decoded back to its
    /// plain bytes.
    #[error("the kept copy of {path} could not be decompressed")]
    Decompress {
        /// The page's address.
        path: PagePath,
        /// What the decoder reported.
        #[source]
        source: std::io::Error,
    },
    /// Accepting or answering connections failed.
    #[error("the server stopped answering requests")]
    Serve {
        /// What the operating system reported.
        #[source]
        source: std::io::Error,
    },
}
