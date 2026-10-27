//! Every check the audit runs, with its group, severity and fix.

/// The groups Ahrefs and Semrush sort their checks into, so an SEO
/// recognises them (`docs/plan.md` section 5.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Category {
    /// Whether every address answers, and how deep it sits.
    Crawlability,
    /// Whether search engines may list a page.
    Indexability,
    /// Titles and meta descriptions.
    Titles,
    /// The page's main heading.
    Headings,
    /// The text on the page and its size.
    Content,
    /// Links between pages of the blog.
    InternalLinks,
    /// Links to other sites.
    OutgoingLinks,
    /// Redirects inside the blog.
    Redirects,
    /// Images and their descriptions.
    Images,
    /// Stylesheets and scripts.
    Resources,
    /// HTTPS and security headers.
    Https,
    /// Languages and `hreflang`.
    Localization,
    /// Open Graph and Twitter tags.
    Social,
    /// Sitemaps, `robots.txt` and `llms.txt`.
    Sitemaps,
    /// JSON-LD structured data.
    StructuredData,
}

impl Category {
    /// The group's name as the tools spell it.
    pub fn title(self) -> &'static str {
        match self {
            Self::Crawlability => "Crawlability and architecture",
            Self::Indexability => "Indexability",
            Self::Titles => "Titles and descriptions",
            Self::Headings => "Headings",
            Self::Content => "Content and duplicates",
            Self::InternalLinks => "Internal links",
            Self::OutgoingLinks => "Outgoing links",
            Self::Redirects => "Redirects",
            Self::Images => "Images",
            Self::Resources => "Resources",
            Self::Https => "HTTPS and security",
            Self::Localization => "Localization and hreflang",
            Self::Social => "Social tags",
            Self::Sitemaps => "Sitemaps and robots",
            Self::StructuredData => "Structured data",
        }
    }
}

/// How much an issue matters, as both tools grade it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// A fault to fix. A page with one does not count as healthy.
    Error,
    /// Worth fixing soon.
    Warning,
    /// Worth knowing.
    Notice,
}

impl Severity {
    /// The word for it, such as `error`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Notice => "notice",
        }
    }
}

/// Declares [`Check`] from one row per check: its variant, its identity,
/// its group, its severity, its name as the tools give it, and its fix.
macro_rules! checks {
    ($(
        $(#[doc = $doc:literal])+
        $name:ident => $id:literal, $category:ident, $severity:ident,
            $title:literal, $fix:literal;
    )+) => {
        /// One check, named after the issue it finds.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[non_exhaustive]
        pub enum Check {
            $($(#[doc = $doc])+ $name,)+
        }

        impl Check {
            /// Every check, in the order reports list them.
            pub const ALL: &[Self] = &[$(Self::$name),+];

            /// The identity, group, severity, name and fix, kept in one
            /// row so a new check cannot miss one.
            fn about(self) -> (&'static str, Category, Severity, &'static str, &'static str) {
                match self {
                    $(Self::$name => (
                        $id,
                        Category::$category,
                        Severity::$severity,
                        $title,
                        $fix,
                    ),)+
                }
            }
        }
    };
}

impl Check {
    /// The check's identity in reports and CSV files, such as
    /// `title-missing`.
    pub fn id(self) -> &'static str {
        self.about().0
    }

    /// The group it belongs to.
    pub fn category(self) -> Category {
        self.about().1
    }

    /// How much an issue it finds matters.
    pub fn severity(self) -> Severity {
        self.about().2
    }

    /// The issue as the tools name it, such as "Title tag missing or
    /// empty".
    pub fn title(self) -> &'static str {
        self.about().3
    }

    /// What to do about it, in one plain sentence.
    pub fn fix(self) -> &'static str {
        self.about().4
    }
}

checks! {
    /// A page asks search engines not to list it, and to follow its links.
    Noindex => "noindex", Indexability, Notice, "Noindex page",
        "Leave it if the page should stay out of search results, or remove noindex from its robots setting.";
    /// A page asks search engines neither to list it nor follow its links.
    NoindexNofollow => "noindex-nofollow", Indexability, Warning, "Noindex and nofollow page",
        "Remove nofollow from the page's robots setting, so search engines still follow its links.";
    /// A page names another page as its canonical.
    NonCanonical => "non-canonical", Indexability, Notice, "Non-canonical page",
        "Link to the canonical address instead, so readers and search engines land on one page.";
    /// A page has no title, or an empty one.
    TitleMissing => "title-missing", Titles, Error, "Title tag missing or empty",
        "Give the page a title that says what it is about.";
    /// A page has more than one title.
    TitleMultiple => "title-multiple", Titles, Warning, "Multiple title tags",
        "Remove the extra title from the template override, so the page has one.";
    /// A page's title is longer than search results show.
    TitleTooLong => "title-too-long", Titles, Notice, "Title too long",
        "Shorten the title to 70 characters or fewer, so search results show all of it.";
    /// A page has no meta description, or an empty one.
    DescriptionMissing => "description-missing", Titles, Warning, "Meta description tag missing or empty",
        "Write a description of one or two sentences that says what the reader gets.";
    /// A page has more than one meta description.
    DescriptionMultiple => "description-multiple", Titles, Warning, "Multiple meta description tags",
        "Remove the extra description from the template override, so the page has one.";
    /// A page's meta description is longer than search results show.
    DescriptionTooLong => "description-too-long", Titles, Notice, "Meta description too long",
        "Shorten the description to 160 characters or fewer, so search results show all of it.";
    /// A page has no `h1`, or an empty one.
    HeadingMissing => "h1-missing", Headings, Warning, "H1 tag missing or empty",
        "Give the page one main heading that says what it is about.";
    /// A page has more than one `h1`.
    HeadingMultiple => "h1-multiple", Headings, Notice, "Multiple H1 tags",
        "Keep one main heading and make the others subheadings.";
    /// A page does not say which language it is in.
    LanguageMissing => "language-missing", Localization, Warning, "Language not declared",
        "Set the lang attribute on the html element in the template override.";
    /// A page search engines may list has few words.
    LowWordCount => "low-word-count", Content, Notice, "Low word count",
        "Add the detail a reader needs, or join the page to a related one.";
    /// A page's text is a small share of its HTML.
    LowTextRatio => "low-text-ratio", Content, Notice, "Low text to HTML ratio",
        "Add text, or take markup the page does not need out of the template override.";
    /// A page's HTML is larger than search engines read in full.
    HtmlTooLarge => "html-too-large", Content, Warning, "HTML is too large",
        "Split the page, or move large inline data out of it, so it stays under 2 MB.";
    /// A page links to another page of the blog with `nofollow`.
    NofollowInternal => "nofollow-internal", InternalLinks, Warning, "Page has nofollow internal links",
        "Remove nofollow from links to your own pages, so search engines follow them.";
    /// A page has more links than the limit.
    TooManyLinks => "too-many-links", InternalLinks, Warning, "Too many links on the page",
        "Keep the links a reader needs and move the rest to a list page.";
    /// An image has no `alt` attribute.
    AltMissing => "alt-missing", Images, Warning, "Missing alt text",
        "Describe each image in a few words, or give it an empty alt when it is decoration.";
    /// A page served over HTTPS links to an HTTP address.
    HttpLink => "http-link", Https, Warning, "HTTPS page has links to HTTP",
        "Change the link to its https address, which most sites answer.";
    /// A page served over HTTPS loads a file over HTTP.
    MixedContent => "mixed-content", Https, Error, "Mixed content",
        "Load the file over https, or keep a copy with the site.";
    /// A page lacks one of the Open Graph tags a shared link needs.
    OpenGraphIncomplete => "open-graph-incomplete", Social, Notice, "Open Graph tags incomplete",
        "Give the page a title, type, image and address for shared links, such as a share image.";
    /// A page has no Twitter card tag.
    TwitterCardMissing => "twitter-card-missing", Social, Notice, "Twitter card missing",
        "Keep the default head, which adds a Twitter card to every page.";
    /// A JSON-LD block is not valid JSON.
    StructuredDataInvalid => "structured-data-invalid", StructuredData, Error,
        "Structured data is not valid JSON",
        "Fix the JSON in the post's schema field or the template override.";
    /// A page has no structured data.
    StructuredDataMissing => "structured-data-missing", StructuredData, Notice, "No structured data",
        "Keep the default head, which describes every page in JSON-LD.";
}
