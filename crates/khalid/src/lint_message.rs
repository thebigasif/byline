//! What a person reads when the linter cannot start (BL-ERR-4).

use khalid_core::UserMessage;
use khalid_lint::LintError;

/// The message for `error`, in what, why and fix.
pub(crate) fn of(_error: &LintError) -> UserMessage {
    UserMessage::new(
        "The linter could not start because its built-in messages are broken.",
        "This is a fault inside Khalid, not in your site.",
        "Report it at https://github.com/thebigasif/khalid/issues with the version number.",
    )
}
