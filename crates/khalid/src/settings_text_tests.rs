//! Settings written into a post's front matter read back the same.

use std::path::Path;

use khalid_api::resources::{
    CustomJsonLd, FeatureImage, NonBlank, PostSettings, PostStatus, ReviewState, SearchTitle,
    ShareCard,
};
use khalid_content::parse_post;
use khalid_core::Slug;

use super::{settings_of, write_settings};
use crate::post_text::PostText;

const FILE: &str = "---\ntitle: Choosing anchor text\nauthor: asif\n---\n\nBody.\n";

/// Writes `after` over the settings of [`FILE`] and reads the file back.
fn round_trip(after: &PostSettings) -> (String, PostSettings) {
    let before = settings_of(&parse_post(Path::new("a.md"), FILE).unwrap().front);
    let mut text = PostText::parse(FILE).unwrap();
    write_settings(&mut text, &before, after, PostStatus::Draft).unwrap();
    let written = text.to_text();
    let read = settings_of(&parse_post(Path::new("a.md"), &written).unwrap().front);
    (written, read)
}

/// Text that is not blank.
fn text(words: &str) -> NonBlank {
    NonBlank::try_new(words).unwrap()
}

#[test]
fn co_authors_image_review_state_and_share_cards_read_back_the_same() {
    let mut after = PostSettings::default();
    after.co_authors = vec![Slug::try_new("lena-vogt").unwrap()];
    let mut image = FeatureImage::new(text("cover.jpg"), text("Two \"linked\" pages"));
    image.caption = Some(text("Links: the short version."));
    after.feature_image = Some(image);
    after.review_state = ReviewState::InReview;
    let mut card = ShareCard::default();
    card.title = Some(text("Anchor text in five minutes"));
    card.image = Some(text("card.png"));
    after.share_cards.x = Some(card);
    let (written, read) = round_trip(&after);
    assert_eq!(read, after, "{written}");
    assert!(written.contains("co_authors: [lena-vogt]\n"), "{written}");
    assert!(written.contains("review_state: in_review\n"), "{written}");
}

#[test]
fn settings_put_back_to_their_defaults_leave_no_line() {
    let mut set = PostSettings::default();
    set.co_authors = vec![Slug::try_new("lena-vogt").unwrap()];
    set.review_state = ReviewState::Ready;
    set.share_cards.facebook = Some(ShareCard::default());
    let (written, _) = round_trip(&set);
    let mut text = PostText::parse(&written).unwrap();
    let defaults = PostSettings::default();
    write_settings(&mut text, &set, &defaults, PostStatus::Draft).unwrap();
    assert_eq!(text.to_text(), FILE);
}

#[test]
fn search_title_focus_keyword_and_schema_read_back_the_same() {
    let mut after = PostSettings::default();
    let title = "Anchor text: \"say where\" {sep} {site}";
    after.seo_title = Some(SearchTitle::try_new(title).unwrap());
    after.focus_keyword = Some(text("anchor text"));
    let schema = "{\n  \"@type\": \"Event\",\n  \"name\": \"Anchor text clinic\"\n}";
    after.schema = Some(CustomJsonLd::try_new(schema).unwrap());
    let (written, read) = round_trip(&after);
    assert_eq!(read, after, "{written}");
    assert!(
        written.contains("focus_keyword: \"anchor text\"\n"),
        "{written}"
    );
}
