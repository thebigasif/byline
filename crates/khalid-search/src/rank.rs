//! Ranking the pages a query found, best first.

use std::collections::BTreeMap;

use crate::FieldCounts;

/// How much one word in the title counts, against one in the body.
const TITLE_WEIGHT: f64 = 4.0;

/// How much one word in a heading counts, against one in the body.
const HEADINGS_WEIGHT: f64 = 2.0;

/// How quickly more of the same word stops adding to a page's score. The
/// usual BM25 value: the third mention adds far less than the first.
const SATURATION: f64 = 1.2;

/// How much a longer field than the average weakens each mention. The usual
/// BM25 value, between 0 (length does not matter) and 1 (fully).
const LENGTH_EFFECT: f64 = 0.75;

/// The pages search runs over, in one language: how many there are, and how
/// many words their fields hold together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Corpus {
    documents: u32,
    total: FieldCounts,
}

impl Corpus {
    /// `documents` pages, whose fields hold `total` words together.
    pub fn new(documents: u32, total: FieldCounts) -> Self {
        Self { documents, total }
    }
}

/// One page a query term found, as the index stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Posting<K> {
    /// The page, by whatever key the caller knows it by.
    pub document: K,
    /// How many words each field of the page has.
    pub lengths: FieldCounts,
    /// How often the term appears in each field.
    pub counts: FieldCounts,
}

impl<K> Posting<K> {
    /// The page `document`, with `lengths` words per field, where the term
    /// appears `counts` times.
    pub fn new(document: K, lengths: FieldCounts, counts: FieldCounts) -> Self {
        Self {
            document,
            lengths,
            counts,
        }
    }
}

/// The pages that hold every query term, best first.
///
/// `postings` holds, for each term of the query in order, the pages it
/// found. A term matched by the start of a word may find a page more than
/// once, once per word; those counts are added. The score is BM25 over the
/// fields: the title counts more than a heading and a heading more than the
/// body, a rare term counts more than a common one, and a long field
/// weakens each mention. Equal scores keep the order of their keys.
pub fn rank<K: Ord + Clone>(corpus: &Corpus, postings: &[Vec<Posting<K>>]) -> Vec<K> {
    let per_term: Vec<BTreeMap<&K, (FieldCounts, FieldCounts)>> =
        postings.iter().map(|found| by_document(found)).collect();
    let Some((first, rest)) = per_term.split_first() else {
        return Vec::new();
    };
    let averages = Averages::of(corpus);
    let mut scored: Vec<(f64, &K)> = first
        .keys()
        .filter(|key| rest.iter().all(|term| term.contains_key(*key)))
        .map(|key| {
            let score = per_term
                .iter()
                .map(|term| {
                    let rarity = rarity(corpus.documents, term.len());
                    let (lengths, counts) = term.get(*key).copied().unwrap_or_default();
                    rarity * saturate(averages.weighted(lengths, counts))
                })
                .sum();
            (score, *key)
        })
        .collect();
    scored.sort_by(|(a_score, a_key), (b_score, b_key)| {
        b_score.total_cmp(a_score).then_with(|| a_key.cmp(b_key))
    });
    scored.into_iter().map(|(_, key)| key.clone()).collect()
}

/// The pages one term found, each once, with its field lengths and the
/// term's counts added up.
fn by_document<K: Ord>(found: &[Posting<K>]) -> BTreeMap<&K, (FieldCounts, FieldCounts)> {
    let mut pages: BTreeMap<&K, (FieldCounts, FieldCounts)> = BTreeMap::new();
    for posting in found {
        let page = pages
            .entry(&posting.document)
            .or_insert((posting.lengths, FieldCounts::default()));
        page.1 = page.1.plus(posting.counts);
    }
    pages
}

/// How much finding a term says about a page: more when few of the
/// `documents` pages hold it, as BM25's inverse document frequency.
fn rarity(documents: u32, holding: usize) -> f64 {
    let holding = f64::from(u32::try_from(holding).unwrap_or(u32::MAX));
    let documents = f64::from(documents).max(holding);
    (1.0 + (documents - holding + 0.5) / (holding + 0.5)).ln()
}

/// A page's weighted count, turned into a score that grows ever more
/// slowly with more mentions.
fn saturate(weighted: f64) -> f64 {
    weighted * (SATURATION + 1.0) / (weighted + SATURATION)
}

/// The average number of words in each field, over the corpus.
#[derive(Debug, Clone, Copy)]
struct Averages {
    title: f64,
    headings: f64,
    body: f64,
}

impl Averages {
    fn of(corpus: &Corpus) -> Self {
        let documents = f64::from(corpus.documents.max(1));
        Self {
            title: f64::from(corpus.total.title) / documents,
            headings: f64::from(corpus.total.headings) / documents,
            body: f64::from(corpus.total.body) / documents,
        }
    }

    /// The term's counts in each field, each weakened by how much longer
    /// than average the field is, then weighted by the field and added.
    fn weighted(&self, lengths: FieldCounts, counts: FieldCounts) -> f64 {
        let field = |count: u32, length: u32, average: f64| {
            let relative = if average > 0.0 {
                f64::from(length) / average
            } else {
                1.0
            };
            f64::from(count) / (1.0 - LENGTH_EFFECT + LENGTH_EFFECT * relative)
        };
        TITLE_WEIGHT * field(counts.title, lengths.title, self.title)
            + HEADINGS_WEIGHT * field(counts.headings, lengths.headings, self.headings)
            + field(counts.body, lengths.body, self.body)
    }
}
