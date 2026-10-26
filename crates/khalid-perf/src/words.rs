//! The words generated posts are made of.
//!
//! A sentence is four slots, each picked from its own list, so a post reads
//! like plain advice and two posts rarely share a run of five words. That
//! keeps the near-copy rule from treating the whole site as one text, which
//! a real blog would not be. Each list is one string, split at `;`.

/// Topics titles are built from, with the slug of each after a `|`.
const TOPICS: &str = "Internal links|internal-links;Title tags|title-tags;Sitemaps|sitemaps;\
    Page speed|page-speed;Topic clusters|topic-clusters;Meta descriptions|meta-descriptions;\
    Image alt text|image-alt-text;Local search|local-search;Product pages|product-pages;\
    Category pages|category-pages;Redirects|redirects;Canonical tags|canonical-tags;\
    Structured data|structured-data;Anchor text|anchor-text;\
    Keyword research|keyword-research;Content audits|content-audits;\
    Broken links|broken-links;Mobile layouts|mobile-layouts;Search intent|search-intent;\
    Author pages|author-pages";

/// How a title approaches its topic, with the slug of each after a `|`.
const ANGLES: &str = "for small shops|small-shops;in one afternoon|one-afternoon;\
    without a developer|without-developer;checklist|checklist;mistakes to avoid|mistakes;\
    explained simply|explained;for new blogs|new-blogs;step by step|step-by-step;\
    on a budget|budget;that last|lasting";

/// Section headings. A post takes four, in an order its seed picks.
const HEADINGS: &str = "Why it matters;Where to start;Common mistakes;A worked example;\
    How to check your work;Tools that help;How long it takes;What to do next";

const SUBJECTS: &str = "A clear title;Every internal link;The page speed report;\
    A short summary;Your sitemap file;A good description;Each category page;The author box;\
    A fresh example;The first paragraph;A tidy web address;The related posts list";

const VERBS: &str = "helps readers find;tells search engines about;makes it easier to reach;\
    gives a clear signal for;saves time when you plan;keeps attention on;\
    points new visitors toward;explains the purpose of;builds trust around;\
    shortens the path to;adds context to;sets expectations for";

const OBJECTS: &str = "the next useful post;the main topic;older articles;the product pages;\
    a deeper guide;the questions people ask;local search results;the checklist below;\
    your best examples;the pricing page;a seasonal offer;the newsletter archive";

const QUALIFIERS: &str = "without extra work;on small screens;for new readers;within a week;\
    in plain words;after each update;before you publish;across the whole site;\
    when traffic grows;for returning visitors;during a redesign;with very little effort";

/// The items of a `;` list, with the line breaks of the source trimmed.
fn items(list: &'static str) -> Vec<&'static str> {
    list.split(';').map(str::trim).collect()
}

/// The words and slug of each topic.
pub(crate) fn topics() -> Vec<(&'static str, &'static str)> {
    pairs(TOPICS)
}

/// The words and slug of each angle.
pub(crate) fn angles() -> Vec<(&'static str, &'static str)> {
    pairs(ANGLES)
}

/// The section headings.
pub(crate) fn headings() -> Vec<&'static str> {
    items(HEADINGS)
}

fn pairs(list: &'static str) -> Vec<(&'static str, &'static str)> {
    items(list)
        .into_iter()
        .filter_map(|item| item.split_once('|'))
        .collect()
}

/// A small, fast generator that gives the same numbers for the same seed on
/// every machine (`SplitMix64`), and the sentence slots it picks from.
#[derive(Debug, Clone)]
pub(crate) struct Seed {
    state: u64,
    slots: [Vec<&'static str>; 4],
}

impl Seed {
    /// The generator for one post.
    pub(crate) fn new(index: usize) -> Self {
        Self {
            state: u64::try_from(index).unwrap_or(u64::MAX),
            slots: [SUBJECTS, VERBS, OBJECTS, QUALIFIERS].map(items),
        }
    }

    /// The next number from the generator.
    pub(crate) fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A position below `len`, picked by the next number.
    pub(crate) fn below(&mut self, len: usize) -> usize {
        let len = u64::try_from(len).unwrap_or(1).max(1);
        usize::try_from(self.next() % len).unwrap_or(0)
    }

    /// One sentence of advice, ending in a full stop.
    pub(crate) fn sentence(&mut self) -> String {
        let mut words = Vec::with_capacity(4);
        for slot in 0..4 {
            let len = self.slots.get(slot).map_or(0, Vec::len);
            let at = self.below(len);
            let word = self.slots.get(slot).and_then(|list| list.get(at));
            words.push(word.copied().unwrap_or_default());
        }
        format!("{}.", words.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_sentence() {
        assert_eq!(Seed::new(7).sentence(), Seed::new(7).sentence());
        assert_ne!(Seed::new(7).sentence(), Seed::new(8).sentence());
    }

    #[test]
    fn every_list_splits_into_whole_items() {
        assert_eq!(topics().len(), 20);
        assert_eq!(angles().len(), 10);
        assert_eq!(headings().len(), 8);
        let sentence = Seed::new(3).sentence();
        assert!(
            !sentence.contains("  ") && !sentence.contains(';'),
            "{sentence}"
        );
    }
}
