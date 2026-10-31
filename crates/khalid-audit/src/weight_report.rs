//! The page-weight meter: how many bytes each page and the files it loads
//! weigh, by kind, heaviest page first.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use khalid_core::AbsoluteUrl;

use crate::report::quoted;
use crate::{Crawl, Outcome, Page, Visit};

/// How many redirects the meter follows to reach a file, so a loop ends.
const MOST_HOPS: usize = 5;

/// What one page weighs: its HTML and each file it loads, once each, as a
/// first visit downloads them. Sizes are the bytes the crawl received,
/// before any compression the server may add.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageWeight {
    url: AbsoluteUrl,
    html: usize,
    styles: usize,
    scripts: usize,
    images: usize,
    other: usize,
    unweighed: usize,
}

impl PageWeight {
    /// The page's address.
    pub fn url(&self) -> &AbsoluteUrl {
        &self.url
    }

    /// The bytes of the page's own HTML.
    pub fn html(&self) -> usize {
        self.html
    }

    /// The bytes of the stylesheets it loads.
    pub fn styles(&self) -> usize {
        self.styles
    }

    /// The bytes of the scripts it loads.
    pub fn scripts(&self) -> usize {
        self.scripts
    }

    /// The bytes of the images it shows.
    pub fn images(&self) -> usize {
        self.images
    }

    /// The bytes of the files it loads that are none of the above.
    pub fn other(&self) -> usize {
        self.other
    }

    /// The bytes of the HTML and every file weighed together.
    pub fn total(&self) -> usize {
        [self.styles, self.scripts, self.images, self.other]
            .into_iter()
            .fold(self.html, usize::saturating_add)
    }

    /// How many files it loads that could not be weighed: those on another
    /// site, which the crawl does not download, and those that are broken.
    pub fn unweighed(&self) -> usize {
        self.unweighed
    }

    /// Adds the file `visit` reached to the kind its content type names.
    fn add(&mut self, visit: &Visit) {
        let kind = visit
            .header("content-type")
            .unwrap_or_default()
            .trim_start()
            .to_ascii_lowercase();
        let sum = if kind.starts_with("text/css") {
            &mut self.styles
        } else if kind.contains("javascript") || kind.contains("ecmascript") {
            &mut self.scripts
        } else if kind.starts_with("image/") {
            &mut self.images
        } else {
            &mut self.other
        };
        *sum = sum.saturating_add(visit.bytes());
    }
}

/// The page-weight meter of one crawl: every HTML page with its weight,
/// heaviest first, and pages of equal weight in crawl order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightReport {
    pages: Vec<PageWeight>,
}

impl WeightReport {
    /// The meter for `crawl`.
    pub fn of(crawl: &Crawl) -> Self {
        let mut pages: Vec<PageWeight> = crawl
            .visits()
            .iter()
            .filter_map(|visit| Some(weigh(crawl, visit, visit.page()?)))
            .collect();
        // A stable sort, so pages of equal weight keep their crawl order.
        pages.sort_by_key(|page| std::cmp::Reverse(page.total()));
        Self { pages }
    }

    /// Every page, heaviest first.
    pub fn pages(&self) -> &[PageWeight] {
        &self.pages
    }

    /// The heaviest page, or `None` when the crawl read no page.
    pub fn heaviest(&self) -> Option<&PageWeight> {
        self.pages.first()
    }

    /// The meter as CSV with a header row, one page per line, heaviest
    /// first, every weight in bytes.
    pub fn csv(&self) -> String {
        let mut csv = String::from("url,total,html,styles,scripts,images,other,unweighed\n");
        for page in &self.pages {
            let _ = writeln!(
                csv,
                "{},{},{},{},{},{},{},{}",
                quoted(page.url.as_str()),
                page.total(),
                page.html,
                page.styles,
                page.scripts,
                page.images,
                page.other,
                page.unweighed
            );
        }
        csv
    }
}

/// The weight of the page `visit` read as `page`.
fn weigh(crawl: &Crawl, visit: &Visit, page: &Page) -> PageWeight {
    let mut weight = PageWeight {
        url: visit.url().clone(),
        html: visit.bytes(),
        styles: 0,
        scripts: 0,
        images: 0,
        other: 0,
        unweighed: 0,
    };
    // A file the page loads twice, such as one image shown twice, is
    // downloaded once.
    let files: BTreeSet<&str> = page.resources().iter().map(AbsoluteUrl::as_str).collect();
    for file in files {
        match reached(crawl, file) {
            Some(file) => weight.add(file),
            None => weight.unweighed = weight.unweighed.saturating_add(1),
        }
    }
    weight
}

/// The visit a browser asking for `url` ends at, through any redirects,
/// when it answered 200 to 299.
fn reached<'a>(crawl: &'a Crawl, url: &str) -> Option<&'a Visit> {
    let mut visit = crawl.visit(url)?;
    for _ in 0..=MOST_HOPS {
        match visit.outcome() {
            Outcome::Redirect { to, .. } => visit = crawl.visit(to.as_str())?,
            outcome => return outcome.is_ok().then_some(visit),
        }
    }
    None
}
