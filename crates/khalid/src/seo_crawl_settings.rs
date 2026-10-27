//! `[seo.crawlers]` and `[seo.feeds]` from `site.toml`: who may crawl the
//! blog and what every feed carries (audit G11, KH-SEO-6, KH-SEO-7).
//! `[seo.links]` becomes the link policy through `LinkPolicy::from`.

use khalid_core::{CrawlerSettings, FeedContentSetting, FeedSettings};
use khalid_seo::{BotAccess, FeedContent, MetaTemplate, RobotsPath, SeoSite, UserAgent};

use crate::seo_settings::{SettingMistake, TEMPLATE};

/// The most posts a feed may hold. Feed readers fetch the whole file on
/// every check, so a longer feed costs every subscriber.
const MOST_FEED_POSTS: u32 = 1000;

/// Checks the crawler rules and sets them on `site`.
///
/// # Errors
///
/// A path or crawler name `khalid-seo` refuses, or a crawler that is both
/// allowed and blocked.
pub(crate) fn crawlers(
    settings: &CrawlerSettings,
    site: &mut SeoSite,
) -> Result<(), SettingMistake> {
    const TABLE: &str = "seo.crawlers";
    for path in &settings.disallow {
        let path = RobotsPath::try_new(path)
            .map_err(|error| SettingMistake::new(TABLE, "disallow", &error, None))?;
        site.crawlers.disallow.push(path);
    }
    let named = [
        ("allow", &settings.allow, BotAccess::Allowed),
        ("block", &settings.block, BotAccess::Blocked),
    ];
    for (key, agents, access) in named {
        for agent in agents {
            let agent = UserAgent::try_new(agent)
                .map_err(|error| SettingMistake::new(TABLE, key, &error, None))?;
            if site.crawlers.bots.contains_key(&agent) {
                let problem = format!("{} is in both allow and block", agent.as_str());
                return Err(SettingMistake::new(TABLE, key, &problem, None));
            }
            site.crawlers.bots.insert(agent, access);
        }
    }
    Ok(())
}

/// Checks the feed options and sets them on `site`.
///
/// # Errors
///
/// A limit outside 1 to 1000, or a source line that is not a template.
pub(crate) fn feeds(settings: &FeedSettings, site: &mut SeoSite) -> Result<(), SettingMistake> {
    const TABLE: &str = "seo.feeds";
    if !(1..=MOST_FEED_POSTS).contains(&settings.limit) {
        let problem = format!(
            "a feed of {} posts is {}",
            settings.limit,
            if settings.limit == 0 {
                "empty"
            } else {
                "too long"
            }
        );
        return Err(SettingMistake::new(
            TABLE,
            "limit",
            &problem,
            Some("a number from 1 to 1000"),
        ));
    }
    let limit = usize::try_from(settings.limit).map_err(|_| {
        SettingMistake::new(
            TABLE,
            "limit",
            &"the number is too large",
            Some("a number from 1 to 1000"),
        )
    })?;
    let source_line = settings
        .source_line
        .as_deref()
        .map(MetaTemplate::parse)
        .transpose()
        .map_err(|error| SettingMistake::new(TABLE, "source_line", &error, Some(TEMPLATE)))?;
    let feeds = &mut site.feeds;
    feeds.content = match settings.content {
        FeedContentSetting::Full => FeedContent::Full,
        FeedContentSetting::Excerpt => FeedContent::Excerpt,
    };
    feeds.before_item.clone_from(&settings.before_item);
    feeds.after_item.clone_from(&settings.after_item);
    feeds.source_line = source_line;
    feeds.limit = limit;
    Ok(())
}

#[cfg(test)]
mod tests {
    use khalid_seo::{BotAccess, LinkTarget, UserAgent};

    use crate::seo_settings::apply;
    use crate::seo_settings::testing::{settings, site};

    #[test]
    fn a_crawler_both_allowed_and_blocked_is_refused() {
        let tables = "[seo.crawlers]\nallow = [\"GPTBot\"]\nblock = [\"GPTBot\"]\n";
        let error = apply(&settings(tables), &mut site()).unwrap_err();
        assert_eq!(error.key, "block");
        assert_eq!(error.problem, "GPTBot is in both allow and block");
    }

    #[test]
    fn blocked_crawlers_and_new_tab_links_reach_the_site() {
        let tables = "[seo.crawlers]\nblock = [\"CCBot\"]\n[seo.links]\nopen = \"new-tab\"\n";
        let mut site = site();
        apply(&settings(tables), &mut site).unwrap();
        let ccbot = UserAgent::try_new("CCBot").unwrap();
        assert_eq!(site.crawlers.bots.get(&ccbot), Some(&BotAccess::Blocked));
        assert_eq!(site.links.external_target, LinkTarget::NewTab);
    }

    #[test]
    fn a_feed_longer_than_1000_posts_is_refused() {
        let error = apply(&settings("[seo.feeds]\nlimit = 1001\n"), &mut site()).unwrap_err();
        assert_eq!(
            error.problem,
            "a feed of 1001 posts is too long, expected a number from 1 to 1000"
        );
    }
}
