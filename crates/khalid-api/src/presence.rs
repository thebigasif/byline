//! Who has each post open in the editor, and which of them holds its lock,
//! so two writers never type over each other (plan 4.6 and 5.12, KH-EDT-2).
//!
//! The editor checks in while a post is open. The first to check in holds
//! the lock; the rest may read. Someone who stops checking in, because the
//! tab closed or the network went, lets go after [`LEASE_SECONDS`]. All of
//! it lives in memory: a restart lets everyone go, and the next check-in
//! takes the lock again.

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// How long someone stays on a post after their last check-in. The admin
/// checks in every 20 seconds, so a closed tab lets go within a minute.
pub const LEASE_SECONDS: i64 = 60;

/// What a check-in asks for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Claim {
    /// Joins the post: takes the lock if nobody holds it, else reads.
    #[default]
    Join,
    /// Takes the lock from whoever holds it, who may then only read.
    TakeOver,
}

/// Someone with a post open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct PresentEditor {
    /// Who, as the post's `updated_by` names them.
    who: String,
    /// Since when: when they opened the post, or took over its lock.
    since: DateTime<Utc>,
}

impl PresentEditor {
    /// Who, as the post's `updated_by` names them.
    pub fn who(&self) -> &str {
        &self.who
    }
}

/// Who has a post open, as the one checking in sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct PostPresence {
    /// Who holds the lock: the one person who may change the post now.
    holder: PresentEditor,
    /// Whether the one checking in holds it.
    yours: bool,
    /// Everyone else with the post open, the earliest first.
    others: Vec<PresentEditor>,
}

impl PostPresence {
    /// Who holds the lock.
    pub fn holder(&self) -> &PresentEditor {
        &self.holder
    }

    /// Whether the one checking in holds it.
    pub fn yours(&self) -> bool {
        self.yours
    }

    /// Everyone else with the post open, the earliest first.
    pub fn others(&self) -> &[PresentEditor] {
        &self.others
    }
}

/// One person on a post, and when they last checked in.
#[derive(Debug)]
struct Seat {
    editor: PresentEditor,
    seen: DateTime<Utc>,
}

impl Seat {
    fn new(who: &str, now: DateTime<Utc>) -> Self {
        let who = who.to_owned();
        let editor = PresentEditor { who, since: now };
        Self { editor, seen: now }
    }

    fn is(&self, who: &str) -> bool {
        self.editor.who == who
    }

    fn here_at(&self, now: DateTime<Utc>) -> bool {
        now - self.seen < TimeDelta::seconds(LEASE_SECONDS)
    }
}

/// Everyone on one post: who holds the lock, then the rest in the order
/// they came.
#[derive(Debug)]
struct Room {
    holder: Seat,
    others: Vec<Seat>,
}

impl Room {
    /// The seats in order, the holder first. `None` when nobody is left.
    fn from_seats(seats: impl IntoIterator<Item = Seat>) -> Option<Self> {
        let mut seats = seats.into_iter();
        let holder = seats.next()?;
        let others = seats.collect();
        Some(Self { holder, others })
    }

    fn into_seats(self) -> impl Iterator<Item = Seat> {
        std::iter::once(self.holder).chain(self.others)
    }

    /// Everyone still here at `now`; the next in line holds the lock when
    /// the holder has gone.
    fn here_at(self, now: DateTime<Utc>) -> Option<Self> {
        Self::from_seats(self.into_seats().filter(|seat| seat.here_at(now)))
    }

    /// The room after `who` checks in at `now` asking for `claim`.
    fn with(self, who: &str, claim: Claim, now: DateTime<Utc>) -> Self {
        let mut seats: Vec<Seat> = self.into_seats().collect();
        let place = seats.iter().position(|seat| seat.is(who));
        let mut seat = match place {
            Some(at) => seats.remove(at),
            None => Seat::new(who, now),
        };
        seat.seen = now;
        match (claim, place) {
            (Claim::Join, None) => seats.push(seat),
            (Claim::Join, Some(at)) | (Claim::TakeOver, Some(at @ 0)) => seats.insert(at, seat),
            (Claim::TakeOver, _) => {
                seat.editor.since = now;
                seats.insert(0, seat);
            }
        }
        let room = Self::from_seats(seats);
        room.unwrap_or_else(|| Self {
            holder: Seat::new(who, now),
            others: Vec::new(),
        })
    }

    /// The room as `who` sees it.
    fn seen_by(&self, who: &str) -> PostPresence {
        let others = self.others.iter().filter(|seat| !seat.is(who));
        PostPresence {
            holder: self.holder.editor.clone(),
            yours: self.holder.is(who),
            others: others.map(|seat| seat.editor.clone()).collect(),
        }
    }
}

/// Who has each post open, by the post's identity.
#[derive(Debug, Default)]
pub struct Presence(Mutex<BTreeMap<String, Room>>);

impl Presence {
    /// Nobody on any post yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Notes that `who` has post `id` open at `now`, asking for `claim`,
    /// and answers who holds its lock.
    pub fn check_in(&self, id: &str, who: &str, claim: Claim, now: DateTime<Utc>) -> PostPresence {
        // Nothing below can panic, so a poisoned lock still holds whole rooms.
        let mut rooms = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let room = rooms.remove(id).and_then(|room| room.here_at(now));
        let room = match room {
            Some(room) => room.with(who, claim, now),
            None => Room {
                holder: Seat::new(who, now),
                others: Vec::new(),
            },
        };
        let seen = room.seen_by(who);
        rooms.insert(id.to_owned(), room);
        seen
    }

    /// Notes that `who` closed post `id`. The next in line holds its lock.
    pub fn leave(&self, id: &str, who: &str) {
        let mut rooms = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(room) = rooms.remove(id) else {
            return;
        };
        let left = Room::from_seats(room.into_seats().filter(|seat| !seat.is(who)));
        if let Some(room) = left {
            rooms.insert(id.to_owned(), room);
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    const POST: &str = "en.choosing-anchor-text";

    fn at(second: i64) -> DateTime<Utc> {
        let start = Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap();
        start + TimeDelta::seconds(second)
    }

    fn holder(seen: &PostPresence) -> &str {
        seen.holder().who()
    }

    fn others(seen: &PostPresence) -> Vec<&str> {
        seen.others().iter().map(PresentEditor::who).collect()
    }

    #[test]
    fn the_first_to_open_a_post_holds_its_lock_and_the_next_reads() {
        let presence = Presence::new();
        let maria = presence.check_in(POST, "maria", Claim::Join, at(0));
        assert!(maria.yours());
        let lena = presence.check_in(POST, "lena", Claim::Join, at(5));
        assert!(!lena.yours());
        assert_eq!(holder(&lena), "maria");
        assert_eq!(others(&lena), Vec::<&str>::new());
        let maria = presence.check_in(POST, "maria", Claim::Join, at(20));
        assert!(maria.yours());
        assert_eq!(others(&maria), ["lena"]);
        assert_eq!(
            maria.holder().since,
            at(0),
            "a check-in keeps the time maria came"
        );
    }

    #[test]
    fn taking_over_moves_the_lock_and_the_holder_reads_after() {
        let presence = Presence::new();
        presence.check_in(POST, "maria", Claim::Join, at(0));
        presence.check_in(POST, "lena", Claim::Join, at(5));
        let lena = presence.check_in(POST, "lena", Claim::TakeOver, at(10));
        assert!(lena.yours());
        assert_eq!(lena.holder().since, at(10));
        assert_eq!(others(&lena), ["maria"]);
        let maria = presence.check_in(POST, "maria", Claim::Join, at(15));
        assert!(!maria.yours());
        assert_eq!(holder(&maria), "lena");
        let again = presence.check_in(POST, "lena", Claim::TakeOver, at(20));
        assert_eq!(
            again.holder().since,
            at(10),
            "taking a held lock changes nothing"
        );
    }

    #[test]
    fn the_lock_lets_go_when_its_holder_leaves_or_stops_checking_in() {
        let presence = Presence::new();
        presence.check_in(POST, "maria", Claim::Join, at(0));
        presence.check_in(POST, "lena", Claim::Join, at(1));
        presence.leave(POST, "maria");
        assert!(presence.check_in(POST, "lena", Claim::Join, at(2)).yours());
        presence.check_in(POST, "maria", Claim::Join, at(3));
        let quiet = at(2 + LEASE_SECONDS);
        let maria = presence.check_in(POST, "maria", Claim::Join, quiet);
        assert!(maria.yours(), "lena last checked in a lease ago");
        assert_eq!(others(&maria), Vec::<&str>::new());
    }

    #[test]
    fn posts_are_locked_one_by_one_and_leaving_an_unknown_post_is_nothing() {
        let presence = Presence::new();
        presence.leave(POST, "maria");
        presence.check_in(POST, "maria", Claim::Join, at(0));
        let other = presence.check_in("en.topic-clusters", "lena", Claim::Join, at(1));
        assert!(other.yours());
        presence.leave(POST, "nobody");
        let maria = presence.check_in(POST, "maria", Claim::Join, at(2));
        assert!(maria.yours());
    }
}
