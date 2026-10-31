//! A post's settings as the Admin API reads and writes them (KH-EDT-3).

use khalid_api::resources::{
    Alias, FeatureImage, NonBlank, PostSettings, ReviewState, ShareCard, ShareCards,
};
use khalid_core::Slug;
use serde_json::{Value, json};

#[cfg(test)]
fn text(text: &str) -> NonBlank {
    NonBlank::try_new(text).unwrap()
}

#[test]
fn settings_never_set_are_empty_false_and_draft() {
    let json = serde_json::to_value(PostSettings::default()).unwrap();
    let empty = json!({
        "date": null, "co_authors": [], "excerpt": null, "feature_image": null,
        "featured": false, "review_state": "draft", "reviewed": null, "canonical": null,
        "robots": null,
        "share_cards": {"x": null, "facebook": null}, "aliases": [], "access": "public",
        "password_protected": false, "template": null, "head_code": null,
        "seo_title": null, "focus_keyword": null, "schema": null,
    });
    assert_eq!(json, empty);
}

#[test]
fn settings_are_written_as_the_api_sends_them() {
    let mut settings = PostSettings::default();
    settings.co_authors = vec![Slug::try_new("mira").unwrap()];
    settings.feature_image = Some(FeatureImage::new(text("/media/a.jpg"), text("A link")));
    settings.review_state = ReviewState::InReview;
    settings.robots = Some("noindex".parse().unwrap());
    settings.aliases = vec![Alias::try_new("/2019/05/anchor-text/").unwrap()];
    let mut card = ShareCard::default();
    card.title = Some(text("Anchor text, done right"));
    let mut cards = ShareCards::default();
    cards.x = Some(card);
    settings.share_cards = cards;
    let json = serde_json::to_value(settings).unwrap();
    assert_eq!(json["co_authors"], json!(["mira"]));
    let image = json!({"src": "/media/a.jpg", "alt": "A link", "caption": null});
    assert_eq!(json["feature_image"], image);
    assert_eq!(json["review_state"], "in_review");
    assert_eq!(json["robots"], "noindex, follow");
    assert_eq!(json["aliases"], json!(["/2019/05/anchor-text/"]));
    let x = json!({"title": "Anchor text, done right", "description": null, "image": null});
    assert_eq!(json["share_cards"]["x"], x);
}

#[test]
fn review_states_read_as_they_are_written() {
    for state in [
        ReviewState::Draft,
        ReviewState::InReview,
        ReviewState::Ready,
    ] {
        let json = serde_json::to_value(state).unwrap();
        assert_eq!(json, state.as_str());
        assert_eq!(serde_json::from_value::<ReviewState>(json).unwrap(), state);
    }
    let published = serde_json::from_value::<ReviewState>(json!("published"));
    assert!(
        published.is_err(),
        "published is a status, not a review state"
    );
}

#[test]
fn an_image_reads_with_a_caption_and_refuses_a_blank_alt() {
    let read = |json: Value| serde_json::from_value::<FeatureImage>(json);
    let image = read(json!({"src": "/media/a.jpg", "alt": "A link", "caption": "Seen."}));
    assert_eq!(image.unwrap().caption, Some(text("Seen.")));
    assert!(read(json!({"src": "/media/a.jpg", "alt": " "})).is_err());
    assert!(read(json!({"src": "", "alt": "A link"})).is_err());
    assert!(
        read(json!({"src": "/media/a.jpg"})).is_err(),
        "alt is required"
    );
    assert!(read(json!({"src": "/a.jpg", "alt": "A", "width": 3})).is_err());
}

#[test]
fn cards_and_aliases_refuse_what_cannot_be_right() {
    let card = serde_json::from_value::<ShareCards>(json!({"x": {"title": ""}}));
    assert!(card.is_err(), "a blank title");
    let other = serde_json::from_value::<ShareCards>(json!({"linkedin": {}}));
    assert!(other.is_err(), "only X and Facebook");
    let alias = serde_json::from_value::<Alias>(json!("2019/old/")).unwrap_err();
    assert!(alias.to_string().contains("starting with /"), "{alias}");
    assert!(Alias::try_new("/old title/").is_none());
    assert!(NonBlank::try_new(" \n").is_none());
}
