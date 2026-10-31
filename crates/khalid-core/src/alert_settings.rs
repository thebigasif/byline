//! Who hears about problems on a site and when, from the `[alerts]` table
//! of `site.toml` (KH-GRO-6).

use serde::Deserialize;

use crate::MailSettings;
use crate::mail_settings::is_address;

/// When `khalid serve` raises an alert, and who gets it by email. Every
/// alert also goes to the webhooks that asked for `alert.raised`. Leave
/// the table out for the defaults:
///
/// ```toml
/// [alerts]
/// email = "owner@example.com"
/// not_found_spike = 50
/// stale_after_days = 365
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct AlertSettings {
    /// Where alerts are emailed, through `[mail]`, or `None` for webhooks
    /// alone.
    #[serde(default)]
    pub email: Option<String>,
    /// How many new 404s in one hour make a spike, 50 unless set.
    #[serde(default = "spike")]
    pub not_found_spike: u32,
    /// How many days a live post may go unchanged before it is called
    /// stale, 365 unless set. 0 switches the check off.
    #[serde(default = "stale")]
    pub stale_after_days: u32,
}

impl Default for AlertSettings {
    fn default() -> Self {
        Self {
            email: None,
            not_found_spike: spike(),
            stale_after_days: stale(),
        }
    }
}

fn spike() -> u32 {
    50
}

fn stale() -> u32 {
    365
}

impl AlertSettings {
    /// The key that is wrong and why, given how the site sends `mail`.
    pub(crate) fn mistake(
        &self,
        mail: Option<&MailSettings>,
    ) -> Option<(&'static str, &'static str)> {
        if let Some(email) = &self.email {
            if !is_address(email) {
                return Some((
                    "email",
                    "email is the address alerts go to, such as owner@example.com",
                ));
            }
            if mail.is_none() {
                return Some((
                    "email",
                    "alerts are emailed through [mail], so add a [mail] table or remove email",
                ));
            }
        }
        if self.not_found_spike == 0 {
            return Some((
                "not_found_spike",
                "a spike is at least one new 404 in an hour, so use 1 or more",
            ));
        }
        None
    }
}
