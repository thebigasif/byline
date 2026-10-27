//! The rules of the Redirection plugin, from the JSON file its Import and
//! Export page writes. The rules live in the plugin's own table, so a
//! `WordPress` export file never holds them.

use serde::de::IgnoredAny;
use serde::{Deserialize, Deserializer};

use crate::MoveStatus;

/// The rules of one Redirection export.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Redirection {
    /// Every rule Khalid can answer, in file order.
    pub rules: Vec<RedirectionRule>,
    /// Every rule it cannot, with the reason, so the report can list them.
    pub skipped: Vec<SkippedRule>,
}

/// One rule Khalid can answer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RedirectionRule {
    /// The rule's id in the plugin.
    pub id: u64,
    /// The addresses it matches.
    pub from: RuleSource,
    /// What a visitor gets.
    pub to: RuleTarget,
}

/// The addresses a rule matches.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleSource {
    /// One path, such as `/bread-recipes/`.
    Path(String),
    /// A regular expression over the path, as the plugin wrote it.
    Pattern(String),
}

/// What a visitor to a matched address gets.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleTarget {
    /// A redirect to `to`, a path or an address, which may use `$1` for a
    /// pattern's groups.
    Moved {
        /// Where the visitor goes.
        to: String,
        /// How long the move lasts.
        status: MoveStatus,
    },
    /// 410: the page is gone for good.
    Gone,
    /// 451: the page is withheld for legal reasons.
    Unavailable,
}

/// A rule Khalid cannot answer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SkippedRule {
    /// The rule's id in the plugin.
    pub id: u64,
    /// The address or pattern it matched.
    pub url: String,
    /// Why it is left behind.
    pub reason: SkipReason,
}

/// Why a rule is left behind.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkipReason {
    /// The rule, or its group, was switched off.
    Disabled,
    /// It depends on the visitor, such as whether they are signed in.
    Conditional {
        /// The plugin's name for the condition, such as `login`.
        match_type: String,
    },
    /// It does something other than redirect, such as pass the request on.
    NotARedirect {
        /// The plugin's name for the action, such as `nothing`.
        action: String,
    },
    /// It answers with a status Khalid does not send.
    Status {
        /// The status code.
        code: u16,
    },
    /// It answers 404, which Khalid already does for any unknown address.
    AlreadyNotFound,
    /// It redirects to an empty address.
    NoTarget,
}

/// Why a file could not be read as a Redirection export.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RedirectionError {
    /// The file is not JSON, or not in the shape the plugin writes.
    #[error("the file is not a Redirection export: {source}")]
    NotRedirection {
        /// What the JSON reader found, with the line and column.
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Deserialize)]
struct RawExport {
    redirects: Vec<RawRule>,
    #[serde(default)]
    groups: Vec<RawGroup>,
}

#[derive(Deserialize)]
struct RawGroup {
    id: u64,
    enabled: bool,
}

#[derive(Deserialize)]
struct RawRule {
    id: u64,
    url: String,
    #[serde(default)]
    regex: bool,
    #[serde(deserialize_with = "status_code")]
    action_code: u16,
    action_type: String,
    #[serde(default)]
    action_data: Option<ActionData>,
    match_type: String,
    enabled: bool,
    #[serde(default)]
    group_id: Option<u64>,
}

/// Some versions of the plugin write the status as text.
#[derive(Deserialize)]
#[serde(untagged)]
enum Code {
    Number(u16),
    Text(String),
}

fn status_code<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u16, D::Error> {
    match Code::deserialize(deserializer)? {
        Code::Number(code) => Ok(code),
        Code::Text(text) => text.trim().parse().map_err(serde::de::Error::custom),
    }
}

/// The target of a plain redirect, or anything else, such as the empty
/// list the plugin writes for an error.
#[derive(Deserialize)]
#[serde(untagged)]
enum ActionData {
    Target { url: String },
    Other(IgnoredAny),
}

/// Reads a Redirection JSON export into the rules Khalid can answer and the
/// ones it leaves behind.
///
/// # Errors
///
/// Returns [`RedirectionError::NotRedirection`] when the text is not JSON
/// or not in the shape the plugin writes.
pub fn read_redirection(json: &str) -> Result<Redirection, RedirectionError> {
    let raw: RawExport =
        serde_json::from_str(json).map_err(|source| RedirectionError::NotRedirection { source })?;
    let mut redirection = Redirection::default();
    for rule in raw.redirects {
        let group_off = raw
            .groups
            .iter()
            .any(|group| Some(group.id) == rule.group_id && !group.enabled);
        let outcome = if rule.enabled && !group_off {
            target(&rule)
        } else {
            Err(SkipReason::Disabled)
        };
        match outcome {
            Ok(to) => redirection.rules.push(RedirectionRule {
                id: rule.id,
                from: if rule.regex {
                    RuleSource::Pattern(rule.url)
                } else {
                    RuleSource::Path(rule.url)
                },
                to,
            }),
            Err(reason) => redirection.skipped.push(SkippedRule {
                id: rule.id,
                url: rule.url,
                reason,
            }),
        }
    }
    Ok(redirection)
}

/// What an enabled rule answers, or why Khalid cannot answer it.
fn target(rule: &RawRule) -> Result<RuleTarget, SkipReason> {
    if rule.match_type != "url" {
        return Err(SkipReason::Conditional {
            match_type: rule.match_type.clone(),
        });
    }
    let status = match (rule.action_type.as_str(), rule.action_code) {
        ("url", 301 | 308) => MoveStatus::Permanent,
        ("url", 302 | 303) => MoveStatus::Found,
        ("url", 307) => MoveStatus::Temporary,
        ("error", 410) => return Ok(RuleTarget::Gone),
        ("error", 451) => return Ok(RuleTarget::Unavailable),
        ("error", 404) => return Err(SkipReason::AlreadyNotFound),
        ("url" | "error", code) => return Err(SkipReason::Status { code }),
        (action, _) => {
            return Err(SkipReason::NotARedirect {
                action: action.to_owned(),
            });
        }
    };
    match &rule.action_data {
        Some(ActionData::Target { url }) if !url.trim().is_empty() => Ok(RuleTarget::Moved {
            to: url.trim().to_owned(),
            status,
        }),
        _ => Err(SkipReason::NoTarget),
    }
}
