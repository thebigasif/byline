//! What `khalid plan` tells the person: what, why and fix (BL-ERR-4).

use khalid_core::UserMessage;

/// The posts or their schedules could not be read.
pub(crate) fn unreadable() -> UserMessage {
    UserMessage::new(
        "Khalid could not read the posts or their schedules.",
        "Without them it cannot say which draft goes out when.",
        "Check the site folder and data/site.db can be read, then run `khalid plan` again.",
    )
}
