//! How `khalid` ends: the exit codes from RULES.md BL-CLI-2.

use std::process::ExitCode;

/// The outcome a script or agent reads from the exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exit {
    /// 0: the command did what was asked.
    Success,
    /// 1: something the person must fix stopped the command.
    Failure,
    /// 2: the command line did not match what Khalid expects.
    Usage,
    /// 3: Khalid could not do the work. Nothing the person typed was wrong.
    Internal,
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        match exit {
            Exit::Success => Self::SUCCESS,
            Exit::Failure => Self::from(1),
            Exit::Usage => Self::from(2),
            Exit::Internal => Self::from(3),
        }
    }
}
