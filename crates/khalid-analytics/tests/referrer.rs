//! Where a reader came from, as the group a view is counted under.

use khalid_analytics::ReferrerGroup;

const OWN: Option<&str> = Some("blog.example.com");

fn group(referer: Option<&str>) -> ReferrerGroup {
    ReferrerGroup::of(referer, OWN, None)
}

#[test]
fn the_five_ai_assistants_are_grouped_as_ai_search() {
    for referer in [
        "https://chatgpt.com/",
        "https://chat.openai.com/c/1234",
        "https://www.perplexity.ai/search?q=internal+links",
        "https://gemini.google.com/app",
        "https://copilot.microsoft.com/",
        "https://claude.ai/chat/abc",
    ] {
        assert_eq!(group(Some(referer)), ReferrerGroup::Ai, "{referer}");
    }
}

#[test]
fn a_chatgpt_link_with_no_referrer_is_still_ai_search() {
    let group = ReferrerGroup::of(None, OWN, Some("chatgpt.com"));
    assert_eq!(group, ReferrerGroup::Ai);
}

#[test]
fn search_engines_in_any_country_are_search() {
    for referer in [
        "https://www.google.com/",
        "https://www.google.co.uk/",
        "https://www.bing.com/search?q=x",
        "https://duckduckgo.com/",
        "https://search.brave.com/search?q=x",
        "android-app://com.google.android.googlequicksearchbox/",
    ] {
        assert_eq!(group(Some(referer)), ReferrerGroup::Search, "{referer}");
    }
}

#[test]
fn social_sites_are_social() {
    for referer in [
        "https://t.co/abc",
        "https://www.linkedin.com/feed/",
        "https://old.reddit.com/r/seo/",
        "https://news.ycombinator.com/item?id=1",
        "https://bsky.app/profile/someone",
        "https://l.facebook.com/l.php",
    ] {
        assert_eq!(group(Some(referer)), ReferrerGroup::Social, "{referer}");
    }
}

#[test]
fn no_referrer_is_direct_and_the_own_site_is_internal() {
    assert_eq!(group(None), ReferrerGroup::Direct);
    assert_eq!(group(Some("")), ReferrerGroup::Direct);
    assert_eq!(
        group(Some("https://blog.example.com/blog/")),
        ReferrerGroup::Internal
    );
    assert_eq!(
        group(Some("https://www.blog.example.com/")),
        ReferrerGroup::Internal
    );
}

#[test]
fn anything_else_is_other_and_lookalikes_do_not_pass() {
    for referer in [
        "https://example.org/a-post",
        "not a link",
        "https://notchatgpt.com/",
        "https://claude.ai.example.net/",
        "android-app://com.google.android.gm/",
    ] {
        assert_eq!(group(Some(referer)), ReferrerGroup::Other, "{referer}");
    }
}

#[test]
fn each_group_reads_back_from_its_stored_name() {
    for group in ReferrerGroup::ALL {
        assert_eq!(ReferrerGroup::from_stored(group.as_str()), Some(group));
    }
    assert_eq!(ReferrerGroup::from_stored("ads"), None);
}
