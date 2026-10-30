//! The numbers behind the rules that measure, set in `lint.toml`.

use serde::Deserialize;

/// The limits a site can tune in the `[thresholds]` table of `lint.toml`.
///
/// Each key is written in kebab-case, such as `thin-post-words = 500`. A key
/// the file leaves out keeps the default documented on its field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Thresholds {
    /// A post with fewer words than this is thin. Default 300, about the
    /// length of a short answer with one example.
    pub thin_post_words: usize,
    /// How many links site-wide may use the same words for the same page
    /// before the next one is flagged. Default 3, which leaves room for a
    /// short series of posts.
    pub same_link_text_max: usize,
    /// The longest title, in characters, before search results cut it.
    /// Default 60, about what Google shows on a desktop result.
    pub title_max_chars: usize,
    /// The longest description, in characters, before search results cut it.
    /// Default 160, about what Google shows on a desktop result.
    pub description_max_chars: usize,
    /// The share of a post's words, in percent, one repeated phrase may
    /// fill. Default 3, well above what natural writing reaches.
    pub keyword_density_percent: usize,
    /// The longest web address ending, in characters. Default 60, about ten
    /// short words.
    pub slug_max_chars: usize,
    /// The most clicks from the home page to a post. Default 3, the plan's
    /// limit: deeper pages are visited less by readers and by Google.
    pub max_clicks: usize,
    /// How many months a post may go without an update before it is stale.
    /// Default 12: most facts in a how-to post hold for about a year.
    pub stale_months: u32,
    /// How many months a post may go without a review before it comes up
    /// again (F3, decision D4). Default 6.
    pub review_months: u32,
    /// How much text, in percent, two posts may share before they count as
    /// near copies, measured on runs of five words. Default 50: above that,
    /// the posts read as one text with small edits.
    pub near_duplicate_percent: usize,
    /// The most links one post's body may hold. Default 100: past that, each
    /// link passes little value and readers stop following them.
    pub max_links: usize,
    /// The highest reading index, LIX, a post of 100 words or more may
    /// reach. Default 60, where LIX calls a text very hard in English and
    /// German alike.
    pub hard_reading_index: usize,
    /// The fewest words a standalone answer after a question-style heading
    /// may hold. Default 40, the answer capsule's floor (decision D3).
    pub answer_min_words: usize,
    /// The most words a standalone answer after a question-style heading
    /// may hold. Default 60, the answer capsule's ceiling (decision D3).
    pub answer_max_words: usize,
    /// The largest share in percent two headings may share before they
    /// read as the same heading. Default 70: near-duplicates share most
    /// words with small grammar differences, not merely a topic.
    pub heading_duplicate_percent: usize,
    /// The most words one paragraph may hold before it buries its ideas.
    /// Default 225, about the top of the 75 to 225 word target.
    pub paragraph_max_words: usize,
    /// The fewest times per thousand words the focus keyword should appear
    /// in a post of 100 words or more. Default 5, Rank Math's floor of half
    /// a percent: about once every 200 words.
    pub focus_min_per_thousand_words: usize,
    /// The most times per thousand words the focus keyword may appear
    /// before it reads as stuffing. Default 25, Rank Math's ceiling of two
    /// and a half percent.
    pub focus_max_per_thousand_words: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            answer_min_words: 40,
            answer_max_words: 60,
            heading_duplicate_percent: 70,
            paragraph_max_words: 225,
            thin_post_words: 300,
            same_link_text_max: 3,
            title_max_chars: 60,
            description_max_chars: 160,
            keyword_density_percent: 3,
            slug_max_chars: 60,
            max_clicks: 3,
            stale_months: 12,
            review_months: 6,
            near_duplicate_percent: 50,
            max_links: 100,
            hard_reading_index: 60,
            focus_min_per_thousand_words: 5,
            focus_max_per_thousand_words: 25,
        }
    }
}
