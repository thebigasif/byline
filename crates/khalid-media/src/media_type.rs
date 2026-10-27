//! What an uploaded file is, read from its first bytes, never from its name
//! (RULES.md BL-SEC-6).

use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A megabyte as the size limits count it.
const MB: u64 = 1024 * 1024;

/// A type the media library accepts, written as its `Content-Type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "&'static str")]
#[non_exhaustive]
pub enum MediaType {
    /// A JPEG image.
    Jpeg,
    /// A PNG image.
    Png,
    /// A WebP image.
    Webp,
    /// An MP4 video.
    Mp4,
    /// A `WebM` video.
    Webm,
    /// MP3 audio.
    Mp3,
    /// AAC audio in an MP4 container.
    M4a,
    /// Ogg audio.
    Ogg,
    /// WAV audio.
    Wav,
    /// FLAC audio.
    Flac,
    /// A PDF document.
    Pdf,
    /// A ZIP archive.
    Zip,
    /// UTF-8 text with no markup.
    Text,
}

/// What a file is for, which decides its size limit and how a post shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A picture, shown in an image or gallery block.
    Image,
    /// A video, played in a video block.
    Video,
    /// Sound, played in an audio block.
    Audio,
    /// Anything else, offered in a file download block.
    File,
}

/// Why an upload was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    /// The file has no bytes.
    #[error("the file is empty")]
    Empty,
    /// The file is a program or a script, which could run on a reader's
    /// computer.
    #[error("the file is a program or a script")]
    Program,
    /// The file is HTML, SVG or XML, which can carry scripts that would run
    /// on this site.
    #[error("the file is HTML, SVG or XML")]
    Markup,
    /// The file starts like an accepted image but its pixels cannot be read.
    #[error("the file starts like an image but cannot be read as one")]
    Broken,
    /// The file is none of the accepted types.
    #[error("the file is not an accepted type")]
    Unknown,
    /// The file is larger than its kind allows.
    #[error(
        "the file is larger than {} MB, the most {} may be",
        kind.max_bytes() / MB,
        kind.with_article()
    )]
    TooLarge {
        /// The kind of file it is.
        kind: Kind,
    },
}

impl MediaType {
    /// Every accepted type.
    pub const ALL: [Self; 13] = [
        Self::Jpeg,
        Self::Png,
        Self::Webp,
        Self::Mp4,
        Self::Webm,
        Self::Mp3,
        Self::M4a,
        Self::Ogg,
        Self::Wav,
        Self::Flac,
        Self::Pdf,
        Self::Zip,
        Self::Text,
    ];

    /// The type of the file whose bytes are `bytes`.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Empty`] for no bytes, [`Refusal::Program`] for an
    /// executable or a script, [`Refusal::Markup`] for HTML, SVG or XML, and
    /// [`Refusal::Unknown`] for anything else that is not accepted.
    pub fn sniff(bytes: &[u8]) -> Result<Self, Refusal> {
        if bytes.is_empty() {
            return Err(Refusal::Empty);
        }
        if is_program(bytes) {
            return Err(Refusal::Program);
        }
        if let Some(found) = binary(bytes) {
            return Ok(found);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| Refusal::Unknown)?;
        let plain = |c: char| !c.is_control() || matches!(c, '\t' | '\n' | '\r' | '\u{c}');
        if !text.chars().all(plain) {
            return Err(Refusal::Unknown);
        }
        if text
            .trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n'])
            .starts_with('<')
        {
            return Err(Refusal::Markup);
        }
        Ok(Self::Text)
    }

    /// What the file is for.
    pub fn kind(self) -> Kind {
        self.facts().0
    }

    /// The extension a file of this type is saved with, such as `jpg`.
    pub fn extension(self) -> &'static str {
        self.facts().1
    }

    /// The `Content-Type` a file of this type is served with.
    pub fn mime(self) -> &'static str {
        self.facts().2
    }

    /// The kind, extension and `Content-Type`.
    fn facts(self) -> (Kind, &'static str, &'static str) {
        match self {
            Self::Jpeg => (Kind::Image, "jpg", "image/jpeg"),
            Self::Png => (Kind::Image, "png", "image/png"),
            Self::Webp => (Kind::Image, "webp", "image/webp"),
            Self::Mp4 => (Kind::Video, "mp4", "video/mp4"),
            Self::Webm => (Kind::Video, "webm", "video/webm"),
            Self::Mp3 => (Kind::Audio, "mp3", "audio/mpeg"),
            Self::M4a => (Kind::Audio, "m4a", "audio/mp4"),
            Self::Ogg => (Kind::Audio, "ogg", "audio/ogg"),
            Self::Wav => (Kind::Audio, "wav", "audio/wav"),
            Self::Flac => (Kind::Audio, "flac", "audio/flac"),
            Self::Pdf => (Kind::File, "pdf", "application/pdf"),
            Self::Zip => (Kind::File, "zip", "application/zip"),
            Self::Text => (Kind::File, "txt", "text/plain; charset=utf-8"),
        }
    }
}

/// A `Content-Type` that names none of the accepted types.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0} is not a media type the library accepts")]
pub struct ParseMediaTypeError(String);

impl FromStr for MediaType {
    type Err = ParseMediaTypeError;

    /// Reads a type back from [`MediaType::mime`].
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|found| found.mime() == text)
            .ok_or_else(|| ParseMediaTypeError(text.to_owned()))
    }
}

impl TryFrom<String> for MediaType {
    type Error = ParseMediaTypeError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<MediaType> for &'static str {
    fn from(media_type: MediaType) -> Self {
        media_type.mime()
    }
}

impl Kind {
    /// The largest file of this kind the library takes.
    pub fn max_bytes(self) -> u64 {
        match self {
            Self::Image | Self::File => 20 * MB,
            Self::Audio => 50 * MB,
            Self::Video => 100 * MB,
        }
    }

    /// Checks a file of `bytes` bytes against [`Kind::max_bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::TooLarge`] when the file is larger.
    pub fn check_size(self, bytes: u64) -> Result<(), Refusal> {
        if bytes > self.max_bytes() {
            return Err(Refusal::TooLarge { kind: self });
        }
        Ok(())
    }

    fn with_article(self) -> &'static str {
        match self {
            Self::Image => "an image",
            Self::Video => "a video",
            Self::Audio => "an audio file",
            Self::File => "a file",
        }
    }
}

/// Executables for Linux, Windows, macOS, Java and WebAssembly, and scripts.
fn is_program(bytes: &[u8]) -> bool {
    const HEADS: [&[u8]; 9] = [
        b"\x7FELF",
        b"MZ",
        b"\xFE\xED\xFA\xCE",
        b"\xFE\xED\xFA\xCF",
        b"\xCE\xFA\xED\xFE",
        b"\xCF\xFA\xED\xFE",
        b"\xCA\xFE\xBA\xBE",
        b"\x00asm",
        b"#!",
    ];
    HEADS.iter().any(|head| bytes.starts_with(head))
}

/// The accepted binary type `bytes` starts as, if any.
fn binary(bytes: &[u8]) -> Option<MediaType> {
    let at =
        |from: usize, want: &[u8]| bytes.get(from..from.saturating_add(want.len())) == Some(want);
    let simple: [(&[u8], MediaType); 7] = [
        (b"\xFF\xD8\xFF", MediaType::Jpeg),
        (b"\x89PNG\r\n\x1A\n", MediaType::Png),
        (b"ID3", MediaType::Mp3),
        (b"OggS", MediaType::Ogg),
        (b"fLaC", MediaType::Flac),
        (b"%PDF-", MediaType::Pdf),
        (b"PK\x03\x04", MediaType::Zip),
    ];
    if let Some(&(_, found)) = simple.iter().find(|(head, _)| bytes.starts_with(head)) {
        return Some(found);
    }
    if at(0, b"RIFF") && at(8, b"WEBP") {
        return Some(MediaType::Webp);
    }
    if at(0, b"RIFF") && at(8, b"WAVE") {
        return Some(MediaType::Wav);
    }
    if at(4, b"ftyp") {
        return mp4_brand(bytes.get(8..12)?);
    }
    if at(0, b"\x1A\x45\xDF\xA3") {
        let head = bytes.get(..64).unwrap_or(bytes);
        return head
            .windows(4)
            .any(|w| w == b"webm")
            .then_some(MediaType::Webm);
    }
    // An MPEG audio frame: eleven sync bits, then layer III.
    match bytes {
        [0xFF, second, ..] if second & 0xE0 == 0xE0 && (second >> 1) & 0b11 == 0b01 => {
            Some(MediaType::Mp3)
        }
        _ => None,
    }
}

/// The MP4 type a file's major brand names: audio for `M4A ` and `M4B `,
/// video for the common video brands, and nothing for others, such as
/// HEIC images and `QuickTime` films.
fn mp4_brand(brand: &[u8]) -> Option<MediaType> {
    const VIDEO: [&[u8]; 11] = [
        b"isom", b"iso2", b"iso4", b"iso5", b"iso6", b"mp41", b"mp42", b"avc1", b"dash", b"MSNV",
        b"M4V ",
    ];
    if brand == b"M4A " || brand == b"M4B " {
        return Some(MediaType::M4a);
    }
    VIDEO.contains(&brand).then_some(MediaType::Mp4)
}
