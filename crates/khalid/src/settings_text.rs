//! A post's settings in its front matter: read for the Admin API, and
//! written back field by field when an edit changes them.

use khalid_api::post_store::UpdatePostError;
use khalid_api::resources::{Alias, NonBlank, PostSettings, PostStatus};
use khalid_content::PostFront;

use crate::post_text::{PostText, quoted};

/// Why a setting with no front matter field is refused.
const NO_FIELD: &str = "Post files have no field for it yet, so it would be lost.";

/// The settings `front` holds. The excerpt is the `description`, which
/// cards, feeds and search results already show.
pub(crate) fn settings_of(front: &PostFront) -> PostSettings {
    let mut settings = PostSettings::default();
    settings.date = front.date;
    settings.excerpt = front.description.as_deref().and_then(NonBlank::try_new);
    settings.featured = front.featured;
    settings.robots.clone_from(&front.robots);
    settings.canonical.clone_from(&front.canonical);
    let aliases = front.aliases.iter().filter_map(|path| Alias::try_new(path));
    settings.aliases = aliases.collect();
    settings
}

/// Writes into `text` each setting that differs between `before` and
/// `after`, for a post in `status`, and leaves every other line as it
/// was.
///
/// # Errors
///
/// Returns [`UpdatePostError::Unsupported`], with `text` unchanged, when
/// `after` changes a setting front matter has no field for, or takes the
/// date off a published post, which would take it down.
pub(crate) fn write_settings(
    text: &mut PostText,
    before: &PostSettings,
    after: &PostSettings,
    status: PostStatus,
) -> Result<(), UpdatePostError> {
    let refuse = |setting, why| Err(UpdatePostError::Unsupported { setting, why });
    if after.co_authors != before.co_authors {
        return refuse("co_authors", NO_FIELD);
    }
    if after.feature_image != before.feature_image {
        return refuse("feature_image", NO_FIELD);
    }
    if after.review_state != before.review_state {
        return refuse("review_state", NO_FIELD);
    }
    if after.share_cards != before.share_cards {
        return refuse("share_cards", NO_FIELD);
    }
    if status == PostStatus::Published && before.date.is_some() && after.date.is_none() {
        let why = "A published post needs its date, and one without it would go offline.";
        return refuse("date", why);
    }
    if after.date != before.date {
        let day = after.date.map(|date| date.format("%Y-%m-%d").to_string());
        put(text, "date", day);
    }
    if after.excerpt != before.excerpt {
        let excerpt = after.excerpt.as_ref().map(|text| quoted(text.as_str()));
        put(text, "description", excerpt);
    }
    if after.featured != before.featured {
        put(text, "featured", after.featured.then(|| "true".to_owned()));
    }
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
    if after.aliases != before.aliases {
        let paths: Vec<String> = after.aliases.iter().map(|a| quoted(a.as_str())).collect();
        let list = (!paths.is_empty()).then(|| format!("[{}]", paths.join(", ")));
        put(text, "aliases", list);
    }
    Ok(())
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
