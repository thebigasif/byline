//! The sites an embed may show in a frame, and their player addresses.

use khalid_core::AbsoluteUrl;

/// Every site an `::embed` may show in a frame, in one place, so the Content
/// Security Policy's `frame-src` (task 1.15) allows exactly these origins.
/// Any other address renders as a plain link.
pub const EMBED_PROVIDERS: [EmbedProvider; 6] = [
    EmbedProvider::YouTube,
    EmbedProvider::Vimeo,
    EmbedProvider::Spotify,
    EmbedProvider::SoundCloud,
    EmbedProvider::CodePen,
    EmbedProvider::Loom,
];

/// A site whose player an embed may show in a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmbedProvider {
    /// Videos from `youtube.com`, played from `youtube-nocookie.com`, which
    /// sets no cookie until the reader presses play.
    YouTube,
    /// Vimeo videos.
    Vimeo,
    /// Spotify tracks, albums, playlists, episodes and shows.
    Spotify,
    /// Tracks from `soundcloud.com`.
    SoundCloud,
    /// Pens from `codepen.io`.
    CodePen,
    /// Loom recordings.
    Loom,
}

impl EmbedProvider {
    /// The origin the player loads from, for the `frame-src` policy.
    #[must_use]
    pub const fn frame_origin(self) -> &'static str {
        match self {
            Self::YouTube => "https://www.youtube-nocookie.com",
            Self::Vimeo => "https://player.vimeo.com",
            Self::Spotify => "https://open.spotify.com",
            Self::SoundCloud => "https://w.soundcloud.com",
            Self::CodePen => "https://codepen.io",
            Self::Loom => "https://www.loom.com",
        }
    }

    /// Whether the player is a short audio bar rather than a wide picture.
    pub(crate) const fn is_audio(self) -> bool {
        matches!(self, Self::Spotify | Self::SoundCloud)
    }

    /// The provider of a link a writer pasted, by its host.
    fn of(host: &str) -> Option<Self> {
        let host = host.strip_prefix("www.").unwrap_or(host);
        let host = host.strip_prefix("m.").unwrap_or(host);
        match host {
            "youtube.com" | "youtu.be" => Some(Self::YouTube),
            "vimeo.com" => Some(Self::Vimeo),
            "open.spotify.com" => Some(Self::Spotify),
            "soundcloud.com" => Some(Self::SoundCloud),
            "codepen.io" => Some(Self::CodePen),
            "loom.com" => Some(Self::Loom),
            _ => None,
        }
    }
}

/// The provider and player address for a link a writer pasted, or `None`
/// when the link is not one of a provider's pages.
pub(crate) fn player(src: &AbsoluteUrl) -> Option<(EmbedProvider, String)> {
    let provider = EmbedProvider::of(src.host())?;
    let mut parts = src.segments();
    // A short link, youtu.be/<id>, is the same video as youtube.com/embed/<id>.
    if src.host() == "youtu.be" {
        parts.insert(0, "embed");
    }
    // Every part that reaches the player address is a plain id or name.
    if !parts.iter().all(|part| is_plain(part)) {
        return None;
    }
    let origin = provider.frame_origin();
    let path = match (provider, parts.as_slice()) {
        (EmbedProvider::YouTube, ["watch"]) => {
            let id = src.query_value("v").filter(|id| is_plain(id))?;
            format!("/embed/{id}")
        }
        (EmbedProvider::YouTube, ["shorts" | "embed" | "live", id])
        | (EmbedProvider::Loom, ["share", id]) => format!("/embed/{id}"),
        (EmbedProvider::Vimeo, [id]) if id.bytes().all(|b| b.is_ascii_digit()) => {
            format!("/video/{id}")
        }
        (
            EmbedProvider::Spotify,
            [
                kind @ ("track" | "album" | "playlist" | "episode" | "show"),
                id,
            ],
        ) => format!("/embed/{kind}/{id}"),
        (EmbedProvider::SoundCloud, [user, track]) => {
            format!("/player/?url=https%3A%2F%2Fsoundcloud.com%2F{user}%2F{track}")
        }
        (EmbedProvider::CodePen, [user, "pen", id]) => {
            format!("/{user}/embed/{id}?default-tab=result")
        }
        _ => return None,
    };
    Some((provider, format!("{origin}{path}")))
}

/// Whether `part` holds only letters, digits, `-` and `_`.
fn is_plain(part: &str) -> bool {
    !part.is_empty()
        && part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
