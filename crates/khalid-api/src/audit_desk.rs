//! The desk the Admin API's key check records each change through
//! (KH-EDT-6): one rule covers every route, so content, settings and staff
//! accounts are all logged.

use std::fmt;
use std::sync::Arc;

use axum::extract::{MatchedPath, OriginalUri};
use axum::http::Method;
use khalid_core::Clock;

use crate::audit_log::{Action, AuditLog, Change};
use crate::keys::Principal;

/// Where the Admin API keeps its audit log, and the clock that dates each
/// change. Cloning shares both.
#[derive(Clone)]
pub struct AuditDesk {
    pub(crate) log: Arc<dyn AuditLog>,
    clock: Arc<dyn Clock>,
}

impl fmt::Debug for AuditDesk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuditDesk")
            .field("clock", &self.clock)
            .finish_non_exhaustive()
    }
}

/// A write on its way through the key check, logged once it succeeds.
pub(crate) struct Pending {
    by: String,
    action: Action,
    route: String,
    path: String,
}

impl AuditDesk {
    /// A desk that keeps changes in `log`, dated by `clock`.
    pub fn new(log: Arc<dyn AuditLog>, clock: impl Clock + 'static) -> Self {
        Self {
            log,
            clock: Arc::new(clock),
        }
    }

    /// The change a request would make, or `None` for a read.
    pub(crate) fn pending(
        method: &Method,
        route: Option<&MatchedPath>,
        uri: Option<&OriginalUri>,
        by: &Principal,
    ) -> Option<Pending> {
        Some(Pending {
            by: by.id().as_str().to_owned(),
            action: Action::of(method)?,
            route: route?.as_str().to_owned(),
            path: uri?.path().to_owned(),
        })
    }

    /// Logs `pending`, which succeeded. A log that cannot be written is
    /// reported and does not undo the change it missed.
    pub(crate) async fn keep(&self, pending: Pending) {
        let change = Change::new(
            self.clock.now(),
            &pending.by,
            pending.action,
            &pending.route,
            &pending.path,
        );
        if let Err(error) = self.log.record(&change).await {
            tracing::warn!(%error, path = %change.path, "a change was not written to the audit log");
        }
    }
}
