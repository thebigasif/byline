//! Where a reader came from, as the group a view is counted under.

use khalid_analytics::{AiTool, ReferrerGroup};

const OWN: Option<&str> = Some("blog.example.com");

fn group(referer: Option<&str>) -> ReferrerGroup {
    ReferrerGroup::of(referer, OWN, None)
}

#[test]
fn each_ai_assistant_is_named_as_the_tool_that_sent_the_reader() {
    for (referer, tool) in [
        ("https://chatgpt.com/", AiTool::ChatGpt),
        ("https://chat.openai.com/c/1234", AiTool::ChatGpt),
        ("https://www.perplexity.ai/search?q=x", AiTool::Perplexity),
        ("https://gemini.google.com/app", AiTool::Gemini),
        ("https://copilot.microsoft.com/", AiTool::Copilot),
        ("https://claude.ai/chat/abc", AiTool::Claude),
        ("https://chat.deepseek.com/", AiTool::DeepSeek),
        ("https://chat.mistral.ai/chat", AiTool::LeChat),
        ("https://www.meta.ai/", AiTool::MetaAi),
        ("https://grok.com/", AiTool::Grok),
    ] {
        assert_eq!(group(Some(referer)), ReferrerGroup::Ai(tool), "{referer}");
    }
}

#[test]
fn an_assistant_link_with_no_referrer_is_named_by_its_utm_source() {
    for (source, tool) in [
        ("chatgpt.com", AiTool::ChatGpt),
        ("openai", AiTool::ChatGpt),
        ("perplexity", AiTool::Perplexity),
        ("copilot.microsoft.com", AiTool::Copilot),
        ("claude.ai", AiTool::Claude),
    ] {
        let group = ReferrerGroup::of(None, OWN, Some(source));
        assert_eq!(group, ReferrerGroup::Ai(tool), "{source}");
    }
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
    let tools = AiTool::ALL.into_iter().map(ReferrerGroup::Ai);
    let plain = [
        ReferrerGroup::Direct,
        ReferrerGroup::Internal,
        ReferrerGroup::Search,
        ReferrerGroup::Social,
        ReferrerGroup::Other,
    ];
    for group in plain.into_iter().chain(tools) {
        assert_eq!(ReferrerGroup::from_stored(group.as_str()), Some(group));
    }
    assert_eq!(ReferrerGroup::from_stored("ads"), None);
    assert_eq!(ReferrerGroup::from_stored("ai:ask-jeeves"), None);
}

#[test]
fn ai_views_stored_before_tools_were_named_read_back_as_unnamed() {
    let group = ReferrerGroup::from_stored("ai");
    assert_eq!(group, Some(ReferrerGroup::Ai(AiTool::Unnamed)));
    assert_eq!(ReferrerGroup::Ai(AiTool::ChatGpt).as_str(), "ai:chatgpt");
    assert_eq!(AiTool::ChatGpt.name(), "ChatGPT");
}
