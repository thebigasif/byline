//! A post's settings in its front matter: read for the Admin API, and
//! written back field by field when an edit changes them.

use khalid_api::post_store::UpdatePostError;
use khalid_api::resources::{
    Alias, FeatureImage, NonBlank, PostSettings, PostStatus, ReviewState, ShareCard, ShareCards,
};
use khalid_content::{self as content, PostFront};
use khalid_core::Slug;

use crate::post_text::{PostText, quoted};

/// The settings `front` holds. The excerpt is the `description`, which
/// cards, feeds and search results already show, and the feature image is
/// the lead `image`.
pub(crate) fn settings_of(front: &PostFront) -> PostSettings {
    let mut settings = PostSettings::default();
    settings.date = front.date;
    settings.co_authors.clone_from(&front.co_authors);
    settings.excerpt = front.description.as_deref().and_then(NonBlank::try_new);
    settings.feature_image = front.image.as_ref().and_then(feature_image);
    settings.featured = front.featured;
    settings.review_state = match front.review_state {
        content::ReviewState::InReview => ReviewState::InReview,
        content::ReviewState::Ready => ReviewState::Ready,
        _ => ReviewState::Draft,
    };
    settings.robots.clone_from(&front.robots);
    settings.canonical.clone_from(&front.canonical);
    settings.share_cards.x = front.share_cards.x.as_ref().map(share_card);
    settings.share_cards.facebook = front.share_cards.facebook.as_ref().map(share_card);
    let aliases = front.aliases.iter().filter_map(|path| Alias::try_new(path));
    settings.aliases = aliases.collect();
    settings
}

/// The lead image in the API's shape. Front matter refuses a blank `src`
/// or `alt`, so `None` only for a value made some other way.
fn feature_image(image: &content::LeadImage) -> Option<FeatureImage> {
    let src = NonBlank::try_new(&image.src)?;
    let alt = NonBlank::try_new(&image.alt)?;
    let mut feature = FeatureImage::new(src, alt);
    feature.caption = image.caption.as_deref().and_then(NonBlank::try_new);
    Some(feature)
}

/// One share card in the API's shape.
fn share_card(card: &content::ShareCard) -> ShareCard {
    let mut shared = ShareCard::default();
    shared.title = card.title.as_deref().and_then(NonBlank::try_new);
    shared.description = card.description.as_deref().and_then(NonBlank::try_new);
    shared.image = card.image.as_deref().and_then(NonBlank::try_new);
    shared
}

/// Writes into `text` each setting that differs between `before` and
/// `after`, for a post in `status`, and leaves every other line as it
/// was.
///
/// # Errors
///
/// Returns [`UpdatePostError::Unsupported`], with `text` unchanged, when
/// `after` takes the date off a published post, which would take it down.
pub(crate) fn write_settings(
    text: &mut PostText,
    before: &PostSettings,
    after: &PostSettings,
    status: PostStatus,
) -> Result<(), UpdatePostError> {
    if status == PostStatus::Published && before.date.is_some() && after.date.is_none() {
        let why = "A published post needs its date, and one without it would go offline.";
        return Err(UpdatePostError::Unsupported {
            setting: "date",
            why,
        });
    }
    if after.date != before.date {
        let day = after.date.map(|date| date.format("%Y-%m-%d").to_string());
        put(text, "date", day);
    }
    if after.co_authors != before.co_authors {
        let slugs: Vec<&str> = after.co_authors.iter().map(Slug::as_str).collect();
        put(text, "co_authors", list(&slugs));
    }
    if after.excerpt != before.excerpt {
        let excerpt = after.excerpt.as_ref().map(|text| quoted(text.as_str()));
        put(text, "description", excerpt);
    }
    if after.feature_image != before.feature_image {
        put(text, "image", after.feature_image.as_ref().map(image_text));
    }
    if after.featured != before.featured {
        put(text, "featured", after.featured.then(|| "true".to_owned()));
    }
    if after.review_state != before.review_state {
        let state = (after.review_state != ReviewState::Draft)
            .then(|| after.review_state.as_str().to_owned());
        put(text, "review_state", state);
    }
    write_links(text, before, after);
    Ok(())
}

/// Writes the settings about where the post points and how it is shared:
/// robots, canonical, share cards and aliases.
fn write_links(text: &mut PostText, before: &PostSettings, after: &PostSettings) {
    if after.robots != before.robots {
        let robots = after
            .robots
            .as_ref()
            .map(|robots| quoted(&robots.to_string()));
        put(text, "robots", robots);
    }
    if after.canonical != before.canonical {
        let canonical = after.canonical.as_ref().map(|url| quoted(url.as_str()));
        put(text, "canonical", canonical);
    }
    if after.share_cards != before.share_cards {
        put(text, "share_cards", cards_text(&after.share_cards));
    }
    if after.aliases != before.aliases {
        let paths: Vec<String> = after.aliases.iter().map(|a| quoted(a.as_str())).collect();
        let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
        put(text, "aliases", list(&paths));
    }
}

/// `items` as a YAML list on one line, or `None` when there are none.
fn list(items: &[&str]) -> Option<String> {
    (!items.is_empty()).then(|| format!("[{}]", items.join(", ")))
}

/// `pairs` with a value as a YAML group on one line, or `None` when no
/// pair has one.
fn group(pairs: &[(&str, Option<String>)]) -> Option<String> {
    let set: Vec<String> = pairs
        .iter()
        .filter_map(|(key, value)| value.as_ref().map(|value| format!("{key}: {value}")))
        .collect();
    (!set.is_empty()).then(|| format!("{{{}}}", set.join(", ")))
}

/// The lead image as front matter writes it.
fn image_text(image: &FeatureImage) -> String {
    let caption = image.caption.as_ref().map(|text| quoted(text.as_str()));
    let pairs = [
        ("src", Some(quoted(image.src.as_str()))),
        ("alt", Some(quoted(image.alt.as_str()))),
        ("caption", caption),
    ];
    group(&pairs).unwrap_or_default()
}

/// The share cards as front matter writes them, or `None` when neither
/// card says anything.
fn cards_text(cards: &ShareCards) -> Option<String> {
    let card = |card: Option<&ShareCard>| {
        let card = card?;
        let text = |value: Option<&NonBlank>| value.map(|value| quoted(value.as_str()));
        group(&[
            ("title", text(card.title.as_ref())),
            ("description", text(card.description.as_ref())),
            ("image", text(card.image.as_ref())),
        ])
    };
    group(&[
        ("x", card(cards.x.as_ref())),
        ("facebook", card(cards.facebook.as_ref())),
    ])
}

/// Sets the field `key` to `value`, or removes it when there is none.
pub(crate) fn put(text: &mut PostText, key: &str, value: Option<String>) {
    match value {
        Some(value) => text.set(key, &value),
        None => {
            text.remove(key);
        }
    }
}

#[cfg(test)]
#[path = "settings_text_tests.rs"]
mod tests;
