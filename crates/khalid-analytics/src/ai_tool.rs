//! Which AI assistant sent a reader, named from the address of its site
//! or the `utm_source` on its link.

/// An AI assistant that sends readers to a site with a link in its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AiTool {
    /// `ChatGPT`, by `OpenAI`.
    ChatGpt,
    /// Perplexity.
    Perplexity,
    /// Gemini, by Google.
    Gemini,
    /// Copilot, by Microsoft.
    Copilot,
    /// Claude, by Anthropic.
    Claude,
    /// `DeepSeek`.
    DeepSeek,
    /// Le Chat, by Mistral.
    LeChat,
    /// Meta AI.
    MetaAi,
    /// Grok, by xAI.
    Grok,
    /// You.com.
    You,
    /// Phind.
    Phind,
    /// An assistant counted before Khalid named each one: the views stored
    /// as AI search with no tool.
    Unnamed,
}

/// Each tool's sites: a host matches a site or any address under it.
const SITES: &[(&str, AiTool)] = &[
    ("chatgpt.com", AiTool::ChatGpt),
    ("chat.openai.com", AiTool::ChatGpt),
    ("perplexity.ai", AiTool::Perplexity),
    ("gemini.google.com", AiTool::Gemini),
    ("bard.google.com", AiTool::Gemini),
    ("copilot.microsoft.com", AiTool::Copilot),
    ("copilot.cloud.microsoft", AiTool::Copilot),
    ("claude.ai", AiTool::Claude),
    ("chat.deepseek.com", AiTool::DeepSeek),
    ("chat.mistral.ai", AiTool::LeChat),
    ("meta.ai", AiTool::MetaAi),
    ("grok.com", AiTool::Grok),
    ("you.com", AiTool::You),
    ("phind.com", AiTool::Phind),
];

/// The `utm_source` names assistants put on their links, besides their
/// sites.
const NAMES: &[(&str, AiTool)] = &[
    ("chatgpt", AiTool::ChatGpt),
    ("openai", AiTool::ChatGpt),
    ("perplexity", AiTool::Perplexity),
    ("gemini", AiTool::Gemini),
    ("copilot", AiTool::Copilot),
    ("claude", AiTool::Claude),
];

impl AiTool {
    /// Every tool, in the order a report lists ties.
    pub const ALL: [Self; 12] = [
        Self::ChatGpt,
        Self::Perplexity,
        Self::Gemini,
        Self::Copilot,
        Self::Claude,
        Self::DeepSeek,
        Self::LeChat,
        Self::MetaAi,
        Self::Grok,
        Self::You,
        Self::Phind,
        Self::Unnamed,
    ];

    /// The tool whose site is `host`, a bare host in lower case.
    pub(crate) fn of_host(host: &str) -> Option<Self> {
        SITES
            .iter()
            .find(|(site, _)| is_under(host, site))
            .map(|&(_, tool)| tool)
    }

    /// The tool that `source`, a bare `utm_source` in lower case, names.
    pub(crate) fn of_source(source: &str) -> Option<Self> {
        NAMES
            .iter()
            .find(|(name, _)| *name == source)
            .map(|&(_, tool)| tool)
            .or_else(|| Self::of_host(source))
    }

    /// The tool's name as a person knows it, such as `ChatGPT`.
    pub fn name(self) -> &'static str {
        match self {
            Self::ChatGpt => "ChatGPT",
            Self::Perplexity => "Perplexity",
            Self::Gemini => "Gemini",
            Self::Copilot => "Copilot",
            Self::Claude => "Claude",
            Self::DeepSeek => "DeepSeek",
            Self::LeChat => "Le Chat",
            Self::MetaAi => "Meta AI",
            Self::Grok => "Grok",
            Self::You => "You.com",
            Self::Phind => "Phind",
            Self::Unnamed => "AI assistant, not named",
        }
    }

    /// The group name its readers' views are stored under, such as
    /// `ai:chatgpt`. [`AiTool::Unnamed`] is plain `ai`, as views were
    /// stored before tools were named.
    pub(crate) fn stored(self) -> &'static str {
        match self {
            Self::ChatGpt => "ai:chatgpt",
            Self::Perplexity => "ai:perplexity",
            Self::Gemini => "ai:gemini",
            Self::Copilot => "ai:copilot",
            Self::Claude => "ai:claude",
            Self::DeepSeek => "ai:deepseek",
            Self::LeChat => "ai:le-chat",
            Self::MetaAi => "ai:meta-ai",
            Self::Grok => "ai:grok",
            Self::You => "ai:you",
            Self::Phind => "ai:phind",
            Self::Unnamed => "ai",
        }
    }
}

/// Whether `host` is `site` or an address under it.
pub(crate) fn is_under(host: &str, site: &str) -> bool {
    host.strip_suffix(site)
        .is_some_and(|rest| rest.is_empty() || rest.ends_with('.'))
}
