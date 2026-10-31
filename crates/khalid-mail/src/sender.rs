//! Who a site's mail comes from.

use khalid_core::MailSettings;

/// The address and name every email of a site is sent from, as `[mail]`
/// sets them. A page or a template never picks it, so it cannot be forged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sender {
    address: String,
    name: Option<String>,
}

impl Sender {
    /// Mail from `address`, with `name` shown beside it when there is one.
    /// `[mail]` checks both when `site.toml` is read.
    #[must_use]
    pub fn new(address: String, name: Option<String>) -> Self {
        Self { address, name }
    }

    /// The address alone, such as `news@example.com`.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    /// The name shown beside the address, if there is one.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The sender as a mail header writes it: `"Field Notes"
    /// <news@example.com>`, with any quote or backslash in the name
    /// escaped, or the address alone.
    #[must_use]
    pub fn mailbox(&self) -> String {
        match &self.name {
            Some(name) => {
                let name = name.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{name}\" <{}>", self.address)
            }
            None => self.address.clone(),
        }
    }
}

impl From<&MailSettings> for Sender {
    fn from(settings: &MailSettings) -> Self {
        Self::new(settings.from.clone(), settings.name.clone())
    }
}
