//! How `byline` ends: the exit codes from RULES.md BL-CLI-2.

use std::process::ExitCode;

/// The outcome a script or agent reads from the exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exit {
    /// 0: the command did what was asked.
    Success,
    /// 3: Byline could not do the work. Nothing the person typed was wrong.
    Internal,
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        match exit {
            Exit::Success => Self::SUCCESS,
            Exit::Internal => Self::from(3),
        }
    }
}
