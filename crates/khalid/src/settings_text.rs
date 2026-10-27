//! A post's settings in its front matter: read for the Admin API, and
//! written back field by field when an edit changes them.

use khalid_api::post_store::UpdatePostError;
use khalid_api::resources::{Alias, NonBlank, PostSettings, PostStatus};
use khalid_content::PostFront;
use khalid_core::PostAccess;

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
    settings.access = front.access;
    settings.password.clone_from(&front.password_hash);
    settings.template.clone_from(&front.template);
    settings.head_code.clone_from(&front.head_code);
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
    write_access(text, before, after);
    Ok(())
}

/// Writes the access, password hash, template and head code that differ
/// between `before` and `after`. Public access is the default, so it is
/// written as no line at all.
fn write_access(text: &mut PostText, before: &PostSettings, after: &PostSettings) {
    if after.access != before.access {
        let access = (after.access != PostAccess::Public).then(|| after.access.to_string());
        put(text, "access", access);
    }
    if after.password != before.password {
        let hash = after.password.as_ref().map(|hash| quoted(hash.as_str()));
        put(text, "password_hash", hash);
    }
    if after.template != before.template {
        put(
            text,
            "template",
            after.template.as_ref().map(ToString::to_string),
        );
    }
    if after.head_code != before.head_code {
        let code = after
            .head_code
            .as_ref()
            .map(|code| lines_quoted(&code.to_string()));
        put(text, "head_code", code);
    }
}

/// `text` as a double-quoted YAML string on one line, keeping its line
/// breaks as `\n`, because a line break can matter inside a script.
fn lines_quoted(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
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
