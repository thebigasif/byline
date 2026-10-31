//! The event block: `Event`.

use chrono::{DateTime, Utc};
use khalid_core::AbsoluteUrl;

use crate::schema::local_business::address_node;
use crate::schema::{node_id, publisher_id};
use crate::{Offer, PostalAddress, SchemaNode, SchemaValue, SeoError, SeoSite};

/// Where an event happens: a place people go to, or an address online.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventLocation {
    /// A venue: its name and street address.
    Venue {
        /// The venue, such as `The Mill`.
        name: String,
        /// Its address.
        address: PostalAddress,
    },
    /// An event held online, at this address.
    Online(AbsoluteUrl),
}

/// Whether an event goes ahead as planned, as schema.org's `EventStatusType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum EventStatus {
    /// It goes ahead as planned.
    #[default]
    Scheduled,
    /// It will not happen.
    Cancelled,
    /// It will happen later, on a date not yet known.
    Postponed,
    /// It moved to a new date, which `start` now holds.
    Rescheduled,
    /// It moved from a venue to online.
    MovedOnline,
}

impl EventStatus {
    fn address(self) -> &'static str {
        match self {
            Self::Scheduled => "https://schema.org/EventScheduled",
            Self::Cancelled => "https://schema.org/EventCancelled",
            Self::Postponed => "https://schema.org/EventPostponed",
            Self::Rescheduled => "https://schema.org/EventRescheduled",
            Self::MovedOnline => "https://schema.org/EventMovedOnline",
        }
    }
}

/// An event block: something that happens at a time and a place.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Event {
    /// The event.
    pub name: String,
    /// When it starts.
    pub start: DateTime<Utc>,
    /// Where it happens.
    pub location: EventLocation,
    /// What happens there, in plain text.
    pub description: String,
    /// When it ends, if known.
    pub end: Option<DateTime<Utc>>,
    /// Whether it goes ahead; scheduled unless set.
    pub status: EventStatus,
    /// Pictures of it.
    pub images: Vec<AbsoluteUrl>,
    /// Its ticket price; a free event costs 0.
    pub offer: Option<Offer>,
    /// The person who speaks or performs, if one does.
    pub performer: Option<String>,
}

impl Event {
    /// An event called `name`, starting at `start`, at `location`.
    pub fn new(name: &str, start: DateTime<Utc>, location: EventLocation) -> Self {
        Self {
            name: name.to_owned(),
            start,
            location,
            description: String::new(),
            end: None,
            status: EventStatus::Scheduled,
            images: Vec::new(),
            offer: None,
            performer: None,
        }
    }
}

/// The `Event` at `<page>#event-<number>`, organised by the site's
/// publisher.
pub(crate) fn event_node(
    event: &Event,
    site: &SeoSite,
    page: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let (mode, location) = match &event.location {
        EventLocation::Venue { name, address } => {
            let place = SchemaNode::new("Place")
                .with("name", SchemaValue::text(name))
                .with("address", address_node(address));
            ("OfflineEventAttendanceMode", place)
        }
        EventLocation::Online(url) => {
            let place =
                SchemaNode::new("VirtualLocation").with("url", SchemaValue::Url(url.clone()));
            ("OnlineEventAttendanceMode", place)
        }
    };
    let mut node = SchemaNode::new("Event")
        .with_id(node_id(page, &format!("event-{number}"))?)
        .with("name", SchemaValue::text(&event.name))
        .with("startDate", SchemaValue::DateTime(event.start));
    if let Some(end) = event.end {
        node.set("endDate", SchemaValue::DateTime(end));
    }
    node.set(
        "eventAttendanceMode",
        SchemaValue::Text(format!("https://schema.org/{mode}")),
    );
    node.set("eventStatus", SchemaValue::text(event.status.address()));
    node.set("location", SchemaValue::Node(Box::new(location)));
    if !event.description.is_empty() {
        node.set("description", SchemaValue::text(&event.description));
    }
    if !event.images.is_empty() {
        let images = event.images.iter().cloned().map(SchemaValue::Url);
        node.set("image", SchemaValue::List(images.collect()));
    }
    if let Some(offer) = &event.offer {
        node.set("offers", SchemaValue::Node(Box::new(offer.node())));
    }
    node.set("organizer", SchemaValue::Reference(publisher_id(site)?));
    if let Some(performer) = &event.performer {
        let person = SchemaNode::new("Person").with("name", SchemaValue::text(performer));
        node.set("performer", SchemaValue::Node(Box::new(person)));
    }
    Ok(node)
}
