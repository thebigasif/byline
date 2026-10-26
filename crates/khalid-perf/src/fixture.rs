//! A generated site: how many posts, and the text of each.

use std::fmt::Write as _;
use std::path::PathBuf;

use chrono::{Days, NaiveDate};

use crate::words::{Seed, angles, headings, topics};

/// The authors posts take turns with: slug and name.
pub(crate) const AUTHORS: [(&str, &str); 4] = [
    ("asif", "Asif"),
    ("lena-vogt", "Lena Vogt"),
    ("sam-okafor", "Sam Okafor"),
    ("mei-tanaka", "Mei Tanaka"),
];

/// The categories posts take turns with.
pub(crate) const CATEGORIES: [&str; 3] = ["on-page-seo", "technical-seo", "content-strategy"];

/// Which posts each post links to, counted forward from itself. Every post
/// then has three links in and three out, like a well-linked blog.
const LINK_OFFSETS: [usize; 3] = [1, 2, 3];

/// The newest post's date. Three posts share each day, going back.
const NEWEST: NaiveDate = match NaiveDate::from_ymd_opt(2026, 10, 1) {
    Some(date) => date,
    None => NaiveDate::MIN,
};

/// A generated site of a given number of English posts.
///
/// Post `n` is the same text on every machine, so measures taken on two
/// runners compare like with like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fixture {
    posts: usize,
}

impl Fixture {
    /// The 1,000-post site most budgets are measured on.
    pub const THOUSAND: Self = Self { posts: 1_000 };

    /// The 10,000-post site the full rebuild is measured on.
    pub const TEN_THOUSAND: Self = Self { posts: 10_000 };

    /// A site of `posts` posts, or `None` for none.
    pub fn new(posts: usize) -> Option<Self> {
        (posts > 0).then_some(Self { posts })
    }

    /// How many posts the site holds.
    pub fn posts(self) -> usize {
        self.posts
    }

    /// Post `index`'s file, relative to the site folder.
    pub fn post_path(self, index: usize) -> PathBuf {
        PathBuf::from(format!(
            "content/en/posts/{}-{}.md",
            date(index),
            slug(index)
        ))
    }

    /// Post `index`'s file contents: front matter, then about 600 words of
    /// markdown in four sections, three links to other posts and one to a
    /// source outside the site.
    pub fn post_text(self, index: usize) -> String {
        let mut seed = Seed::new(index);
        let (author, _) = turn(&AUTHORS, index);
        let mut text = format!(
            "---\ntitle: {}\nslug: {}\ndate: {}\nauthor: {author}\ncategory: {}\n\
             description: {} Practical advice for small teams, with one example to copy.\n---\n\n",
            title(index),
            slug(index),
            date(index),
            turn(&CATEGORIES, index),
            seed.sentence(),
        );
        let mut headings = headings();
        for section in 0..4 {
            let heading = headings.swap_remove(seed.below(headings.len()));
            let _ = write!(text, "## {heading}\n\n");
            for _ in 0..3 {
                let sentences: Vec<String> = (0..4).map(|_| seed.sentence()).collect();
                let _ = write!(text, "{}\n\n", sentences.join(" "));
            }
            if let Some(offset) = LINK_OFFSETS.get(section) {
                self.write_link(&mut text, index, *offset);
            }
        }
        text.push_str(
            "Google explains the basics in its \
             [search essentials](https://developers.google.com/search/docs/essentials).\n",
        );
        text
    }

    /// A sentence linking to the post `offset` places after `index`, unless
    /// the site is too small for it to be another post.
    fn write_link(self, text: &mut String, index: usize, offset: usize) {
        if offset >= self.posts {
            return;
        }
        let target = (index + offset) % self.posts;
        let _ = write!(
            text,
            "For more, read [{}](/blog/{}/).\n\n",
            title(target),
            slug(target)
        );
    }
}

/// The item of `list` that post `index` takes, in turn.
fn turn<T: Copy + Default>(list: &[T], index: usize) -> T {
    list.get(index % list.len().max(1))
        .copied()
        .unwrap_or_default()
}

/// The topic, angle and part number of post `index`: each topic once per
/// angle, then the next part.
fn naming(
    index: usize,
) -> (
    (&'static str, &'static str),
    (&'static str, &'static str),
    usize,
) {
    let (topics, angles) = (topics(), angles());
    let topic = turn(&topics, index);
    let angle = turn(&angles, index / topics.len().max(1));
    let part = index / (topics.len() * angles.len()).max(1) + 1;
    (topic, angle, part)
}

fn title(index: usize) -> String {
    let ((topic, _), (angle, _), part) = naming(index);
    format!("{topic} {angle}, part {part}")
}

fn slug(index: usize) -> String {
    let ((_, topic), (_, angle), part) = naming(index);
    format!("{topic}-{angle}-{part}")
}

fn date(index: usize) -> NaiveDate {
    let back = u64::try_from(index / 3).unwrap_or(0);
    NEWEST.checked_sub_days(Days::new(back)).unwrap_or(NEWEST)
}
