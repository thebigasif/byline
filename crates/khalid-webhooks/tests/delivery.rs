//! The delivery log model: the retry schedule over 24 hours, and how each
//! attempt moves a delivery on.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_core::AbsoluteUrl;
use khalid_webhooks::{
    Attempt, Delivery, DeliveryId, DeliveryState, Event, NewDelivery, Outcome, RetrySchedule,
    WebhookId,
};

/// When the first attempt is made in these tests.
const START: i64 = 1_793_001_600;

#[cfg(test)]
fn at(unix: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(unix, 0).single().unwrap()
}

#[cfg(test)]
fn delivery() -> Delivery {
    NewDelivery::new(
        WebhookId::new(7),
        Event::PostPublished,
        AbsoluteUrl::parse("https://hooks.example.com/khalid").unwrap(),
        br#"{"event":"post.published"}"#.to_vec(),
        at(START),
    )
    .into_delivery(DeliveryId::new(1))
}

#[cfg(test)]
fn failed(at: DateTime<Utc>) -> Attempt {
    Attempt::new(at, Outcome::Answered { status: 503 })
}

#[test]
fn the_schedule_spreads_retries_over_a_day() {
    let delays = RetrySchedule::default().delays().to_vec();
    let total: Duration = delays.iter().sum();
    assert!(total <= Duration::from_hours(24), "{total:?}");
    assert!(total >= Duration::from_hours(23), "{total:?}");
    assert!(delays.is_sorted(), "{delays:?}");
}

#[test]
fn the_schedule_gives_the_next_time_then_gives_up() {
    let schedule = RetrySchedule::default();
    assert_eq!(schedule.next(1, at(START)), Some(at(START + 60)));
    assert_eq!(schedule.next(schedule.delays().len() + 1, at(START)), None);
    assert_eq!(schedule.next(0, at(START)), None);
}

#[test]
fn only_a_2xx_answer_is_a_success() {
    for status in [100, 199, 200, 204, 299, 300, 301, 410, 503] {
        let outcome = Outcome::Answered { status };
        let success = (200..300).contains(&status);
        assert_eq!(outcome.is_success(), success, "{status}");
    }
    let unreachable = Outcome::Unreachable {
        reason: "the request took longer than its time limit".to_owned(),
    };
    assert!(!unreachable.is_success());
}

#[test]
fn a_new_delivery_is_due_at_once_with_what_it_carries() {
    let delivery = delivery();
    assert_eq!(delivery.id(), DeliveryId::new(1));
    assert_eq!(delivery.webhook(), WebhookId::new(7));
    assert_eq!(delivery.event(), Event::PostPublished);
    assert_eq!(delivery.url().as_str(), "https://hooks.example.com/khalid");
    assert_eq!(delivery.body(), br#"{"event":"post.published"}"#);
    assert_eq!(
        delivery.state(),
        &DeliveryState::Pending { next_at: at(START) }
    );
    assert_eq!(delivery.attempts(), []);
    assert!(delivery.is_due(at(START)));
    assert!(!delivery.is_due(at(START - 1)));
}

#[test]
fn a_success_marks_the_delivery_delivered() {
    let mut delivery = delivery();
    let answered = Attempt::new(at(START), Outcome::Answered { status: 200 });
    delivery.record(answered.clone(), &RetrySchedule::default());
    assert_eq!(
        delivery.state(),
        &DeliveryState::Delivered { at: at(START) }
    );
    assert_eq!(delivery.attempts(), [answered]);
    assert!(!delivery.is_due(at(START + 86_400)));
}

#[test]
fn a_failure_schedules_a_retry() {
    let mut delivery = delivery();
    delivery.record(failed(at(START)), &RetrySchedule::default());
    assert_eq!(
        delivery.state(),
        &DeliveryState::Pending {
            next_at: at(START + 60)
        }
    );
    assert!(!delivery.is_due(at(START + 59)));
    assert!(delivery.is_due(at(START + 60)));
}

#[test]
fn a_success_on_the_last_try_is_delivered() {
    let schedule = RetrySchedule::default();
    let mut delivery = delivery();
    for _ in schedule.delays() {
        delivery.record(failed(at(START)), &schedule);
    }
    let last = Attempt::new(at(START + 1), Outcome::Answered { status: 202 });
    delivery.record(last, &schedule);
    let delivered = DeliveryState::Delivered { at: at(START + 1) };
    assert_eq!(delivery.state(), &delivered);
}

#[test]
fn every_failure_gives_up_within_a_day() {
    let schedule = RetrySchedule::default();
    let mut delivery = delivery();
    let mut now = at(START);
    while let DeliveryState::Pending { next_at } = *delivery.state() {
        now = next_at;
        delivery.record(failed(now), &schedule);
    }
    assert_eq!(delivery.state(), &DeliveryState::GaveUp { at: now });
    assert_eq!(delivery.attempts().len(), schedule.delays().len() + 1);
    assert!(now - at(START) <= TimeDelta::hours(24), "{now}");
}
