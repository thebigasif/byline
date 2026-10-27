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
}
