//! Every check the audit runs, with its group, severity, reason and fix.

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
    /// Sitemaps and `robots.txt`.
    Sitemaps,
    /// JSON-LD structured data.
    StructuredData,
    /// What AI search tools read, such as `llms.txt`.
    AiSearch,
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
            Self::AiSearch => "AI search",
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
/// its group, its severity, its name as the tools give it, why it
/// matters, and its fix.
macro_rules! checks {
    ($(
        $(#[doc = $doc:literal])+
        $name:ident => $id:literal, $category:ident, $severity:ident,
            $title:literal, $why:literal, $fix:literal;
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

            /// The identity, group, severity, name, reason and fix, kept
            /// in one row so a new check cannot miss one.
            fn about(self) -> About {
                match self {
                    $(Self::$name => About {
                        id: $id,
                        category: Category::$category,
                        severity: Severity::$severity,
                        title: $title,
                        why: $why,
                        fix: $fix,
                    },)+
                }
            }
        }
    };
}

/// What one row of the catalogue says about a check.
struct About {
    id: &'static str,
    category: Category,
    severity: Severity,
    title: &'static str,
    why: &'static str,
    fix: &'static str,
}

impl Check {
    /// The check's identity in reports and CSV files, such as
    /// `title-missing`.
    pub fn id(self) -> &'static str {
        self.about().id
    }

    /// The group it belongs to.
    pub fn category(self) -> Category {
        self.about().category
    }

    /// How much an issue it finds matters.
    pub fn severity(self) -> Severity {
        self.about().severity
    }

    /// What is wrong, as the tools name the issue, such as "Title tag
    /// missing or empty".
    pub fn title(self) -> &'static str {
        self.about().title
    }

    /// Why the issue matters, in one plain sentence.
    pub fn why(self) -> &'static str {
        self.about().why
    }

    /// What to do about it, in one plain sentence.
    pub fn fix(self) -> &'static str {
        self.about().fix
    }
}

checks! {
    /// A page asks search engines not to list it, and to follow its links.
    Noindex => "noindex", Indexability, Notice, "Noindex page",
        "Search engines will leave this page out of their results.",
        "Leave it if the page should stay out of search results, or remove noindex from its robots setting.";
    /// A page asks search engines neither to list it nor follow its links.
    NoindexNofollow => "noindex-nofollow", Indexability, Warning, "Noindex and nofollow page",
        "Search engines will neither list this page nor follow its links to your other pages.",
        "Remove nofollow from the page's robots setting, so search engines still follow its links.";
    /// A page names another page as its canonical.
    NonCanonical => "non-canonical", Indexability, Notice, "Non-canonical page",
        "Search engines list the canonical page instead of this one.",
        "Link to the canonical address instead, so readers and search engines land on one page.";
    /// A page has no title, or an empty one.
    TitleMissing => "title-missing", Titles, Error, "Title tag missing or empty",
        "The title is the line search results and browser tabs show, so without it the page has no name.",
        "Give the page a title that says what it is about.";
    /// A page has more than one title.
    TitleMultiple => "title-multiple", Titles, Warning, "Multiple title tags",
        "Search engines pick one of the titles, and it may not be the one you meant.",
        "Remove the extra title from the template override, so the page has one.";
    /// A page's title is longer than search results show.
    TitleTooLong => "title-too-long", Titles, Notice, "Title too long",
        "Search results cut a long title short, so readers miss its end.",
        "Shorten the title to 70 characters or fewer, so search results show all of it.";
    /// A page has no meta description, or an empty one.
    DescriptionMissing => "description-missing", Titles, Warning, "Meta description tag missing or empty",
        "Search engines write their own snippet from the page, which may not sell it.",
        "Write a description of one or two sentences that says what the reader gets.";
    /// A page has more than one meta description.
    DescriptionMultiple => "description-multiple", Titles, Warning, "Multiple meta description tags",
        "Search engines pick one of the descriptions, and it may not be the one you meant.",
        "Remove the extra description from the template override, so the page has one.";
    /// A page's meta description is longer than search results show.
    DescriptionTooLong => "description-too-long", Titles, Notice, "Meta description too long",
        "Search results cut a long description short, so readers miss its end.",
        "Shorten the description to 160 characters or fewer, so search results show all of it.";
    /// A page has no `h1`, or an empty one.
    HeadingMissing => "h1-missing", Headings, Warning, "H1 tag missing or empty",
        "The main heading tells readers and search engines what the page is about.",
        "Give the page one main heading that says what it is about.";
    /// A page has more than one `h1`.
    HeadingMultiple => "h1-multiple", Headings, Notice, "Multiple H1 tags",
        "Two main headings blur what the page is mainly about.",
        "Keep one main heading and make the others subheadings.";
    /// A page does not say which language it is in.
    LanguageMissing => "language-missing", Localization, Warning, "Language not declared",
        "Screen readers and search engines must guess the language, and may guess wrong.",
        "Set the lang attribute on the html element in the template override.";
    /// A page search engines may list has few words.
    LowWordCount => "low-word-count", Content, Notice, "Low word count",
        "A thin page rarely answers a reader's question in full.",
        "Add the detail a reader needs, or join the page to a related one.";
    /// A page's text is a small share of its HTML.
    LowTextRatio => "low-text-ratio", Content, Notice, "Low text to HTML ratio",
        "Markup with little text loads slowly and gives search engines little to read.",
        "Add text, or take markup the page does not need out of the template override.";
    /// A page's HTML is larger than search engines read in full.
    HtmlTooLarge => "html-too-large", Content, Warning, "HTML is too large",
        "Search engines stop reading a very large page, so its end goes unseen.",
        "Split the page, or move large inline data out of it, so it stays under 2 MB.";
    /// A page links to another page of the blog with `nofollow`.
    NofollowInternal => "nofollow-internal", InternalLinks, Warning, "Page has nofollow internal links",
        "Search engines do not follow these links, so your linked pages get less weight.",
        "Remove nofollow from links to your own pages, so search engines follow them.";
    /// A page has more links than the limit.
    TooManyLinks => "too-many-links", InternalLinks, Warning, "Too many links on the page",
        "Each link counts for less when a page has very many of them.",
        "Keep the links a reader needs and move the rest to a list page.";
    /// An image has no `alt` attribute.
    AltMissing => "alt-missing", Images, Warning, "Missing alt text",
        "Screen readers and search engines cannot see an image, so they read its alt text instead.",
        "Describe each image in a few words, or give it an empty alt when it is decoration.";
    /// A page served over HTTPS links to an HTTP address.
    HttpLink => "http-link", Https, Warning, "HTTPS page has links to HTTP",
        "A reader who follows the link leaves the secure connection.",
        "Change the link to its https address, which most sites answer.";
    /// A page served over HTTPS loads a file over HTTP.
    MixedContent => "mixed-content", Https, Error, "Mixed content",
        "Browsers block or warn about files loaded without HTTPS, so the page may break.",
        "Load the file over https, or keep a copy with the site.";
    /// A page lacks one of the Open Graph tags a shared link needs.
    OpenGraphIncomplete => "open-graph-incomplete", Social, Notice, "Open Graph tags incomplete",
        "A shared link without these tags shows a plain card that few people click.",
        "Give the page a title, type, image and address for shared links, such as a share image.";
    /// A page has no Twitter card tag.
    TwitterCardMissing => "twitter-card-missing", Social, Notice, "Twitter card missing",
        "A link shared on X shows as plain text without a card.",
        "Keep the default head, which adds a Twitter card to every page.";
    /// A JSON-LD block is not valid JSON.
    StructuredDataInvalid => "structured-data-invalid", StructuredData, Error,
        "Structured data is not valid JSON",
        "Search engines ignore structured data they cannot read.",
        "Fix the JSON in the post's schema field or the template override.";
    /// A page has no structured data.
    StructuredDataMissing => "structured-data-missing", StructuredData, Notice, "No structured data",
        "Structured data tells search engines what the page is, such as an article and its author.",
        "Keep the default head, which describes every page in JSON-LD.";
    /// A page the blog links to answers 400 to 499.
    Page4xx => "page-4xx", Crawlability, Error, "4XX page",
        "Readers and search engines who reach this address find an error instead of a page.",
        "Bring the page back, or redirect its address to the closest page and fix the links to it.";
    /// A page the blog links to answers 500 or above.
    Page5xx => "page-5xx", Crawlability, Error, "5XX page",
        "The server failed on this address, so nobody can read the page.",
        "Check the server log for this address and fix what fails there.";
    /// A page the blog links to gave no answer.
    PageUnreached => "page-unreached", Crawlability, Error, "Page timed out or failed",
        "A page that does not answer cannot be read or listed.",
        "Check the server answers this address quickly, then run the audit again.";
    /// A page sits more than three clicks from the home page.
    TooDeep => "too-deep", Crawlability, Notice, "Page is more than 3 clicks from home",
        "Pages far from home are found less often by readers and search engines.",
        "Link to the page from a page closer to home, such as a pillar post or a list.";
    /// A page links to a page of the blog that is broken.
    BrokenLink => "broken-link", InternalLinks, Error, "Page has links to broken page",
        "A reader who follows the link lands on an error page.",
        "Change the link to a page that exists, or remove it.";
    /// Only one other page links to a page, without `nofollow`.
    OneInboundLink => "one-inbound-link", InternalLinks, Notice,
        "Page has only one dofollow incoming internal link",
        "A page with one link to it is easy to miss for readers and search engines.",
        "Link to the page from one or two related posts.";
    /// A page links to an address of the blog that redirects.
    LinksToRedirect => "links-to-redirect", Redirects, Notice, "Page has links to redirect",
        "Each redirect adds a wait for the reader and a step for search engines.",
        "Change the link to the address it redirects to.";
    /// A redirect leads to another redirect.
    RedirectChain => "redirect-chain", Redirects, Warning, "Redirect chain",
        "Every extra step slows the reader, and search engines may stop following.",
        "Point the first redirect straight at the last address.";
    /// Redirects lead back to an address already passed.
    RedirectLoop => "redirect-loop", Redirects, Error, "Redirect loop",
        "A loop never reaches a page, so the browser gives up with an error.",
        "Remove one of the redirects so the chain ends at a page.";
    /// An image a page shows is broken.
    ImageBroken => "image-broken", Images, Error, "Image broken",
        "Readers see an empty box where the image should be.",
        "Put the image back, or change the page to one that exists.";
    /// A stylesheet or script a page loads is broken.
    ResourceBroken => "resource-broken", Resources, Error, "CSS or JavaScript file broken",
        "Without the file the page may look or work wrong.",
        "Build the site again, so every file a page loads is there.";
    /// A page's canonical points at a broken page.
    CanonicalBroken => "canonical-broken", Indexability, Error, "Canonical points to 4XX or 5XX page",
        "Search engines are told to list a page that does not work, so they may list neither.",
        "Point the canonical at a page that answers, usually the page itself.";
    /// A page's canonical points at a redirect.
    CanonicalRedirect => "canonical-redirect", Indexability, Error, "Canonical points to redirect",
        "Search engines get two different answers about which page to list.",
        "Point the canonical at the address the redirect leads to.";
    /// Two listed pages share a title.
    TitleDuplicate => "title-duplicate", Titles, Warning, "Duplicate title tags",
        "Pages with the same title compete with each other in search results.",
        "Give each page its own title, so search engines can tell them apart.";
    /// Two listed pages share a meta description.
    DescriptionDuplicate => "description-duplicate", Titles, Warning, "Duplicate meta descriptions",
        "Readers cannot tell the pages apart in search results.",
        "Write a description for each page that says what that page offers.";
    /// A page names translations but not itself.
    HreflangNoSelf => "hreflang-no-self", Localization, Warning, "Missing self-referencing hreflang",
        "Search engines may ignore the page's list of translations.",
        "Keep the default head, which names the page itself among its translations.";
    /// A translation a page names is broken.
    HreflangBroken => "hreflang-broken", Localization, Error, "Hreflang to broken page",
        "Readers of that language are sent to an error page.",
        "Point the hreflang at the translation's address, or remove it.";
    /// A translation a page names redirects.
    HreflangRedirect => "hreflang-redirect", Localization, Warning, "Hreflang to redirect",
        "Search engines may ignore a translation that sits behind a redirect.",
        "Point the hreflang at the address the redirect leads to.";
    /// A translation does not name the page back.
    HreflangNoReturn => "hreflang-no-return", Localization, Warning, "Missing reciprocal hreflang",
        "Search engines ignore a translation link that only points one way.",
        "Name each translation on every other one, so the pair points both ways.";
    /// The host has no `robots.txt`.
    RobotsMissing => "robots-missing", Sitemaps, Notice, "Robots.txt not found",
        "Robots.txt is where crawlers look first for your rules and your sitemap.",
        "Put the robots.txt Khalid writes in _root at the root of your site.";
    /// The blog has no sitemap.
    SitemapMissing => "sitemap-missing", Sitemaps, Warning, "Sitemap not found",
        "A sitemap lists every page, so search engines find new posts sooner.",
        "Keep the sitemap.xml Khalid writes, and name it in robots.txt.";
    /// A sitemap `robots.txt` names does not answer.
    SitemapBroken => "sitemap-broken", Sitemaps, Error, "Sitemap file broken",
        "Search engines were told to read this sitemap and cannot.",
        "Change the Sitemap line in robots.txt to the address of the sitemap.";
    /// A sitemap lists a broken page.
    SitemapBrokenPage => "sitemap-broken-page", Sitemaps, Error, "4XX or 5XX page in sitemap",
        "Search engines spend their visits on a page that does not work.",
        "Build the site again, so the sitemap lists only pages that exist.";
    /// A sitemap lists an address that redirects.
    SitemapRedirect => "sitemap-redirect", Sitemaps, Warning, "3XX redirect in sitemap",
        "A sitemap should list only the final addresses, or search engines trust it less.",
        "List the address the redirect leads to instead.";
    /// A sitemap lists a page search engines are asked not to list.
    SitemapNoindex => "sitemap-noindex", Sitemaps, Warning, "Noindex page in sitemap",
        "The sitemap asks search engines to list a page the page itself asks them not to list.",
        "Take the page out of the sitemap, or remove noindex from it.";
    /// A sitemap lists a page whose canonical is another page.
    SitemapNonCanonical => "sitemap-non-canonical", Sitemaps, Warning, "Non-canonical page in sitemap",
        "The sitemap and the page disagree about which address to list.",
        "List the canonical address instead.";
    /// Only a sitemap leads to a page: no page links to it.
    Orphan => "orphan", InternalLinks, Warning, "Orphan page (has no incoming internal links)",
        "Readers cannot reach the page by clicking, and search engines give it little weight.",
        "Link to the page from a related post, a list or the home page.";
    /// The blog has no `llms.txt`.
    LlmsMissing => "llms-missing", AiSearch, Notice, "llms.txt not found",
        "Some AI tools read llms.txt for a short map of the site.",
        "Keep the llms.txt Khalid writes beside the home page.";
    /// The blog's `llms.txt` does not start with a `#` title.
    LlmsFormat => "llms-format", AiSearch, Warning, "llms.txt format is not valid",
        "Tools that read llms.txt expect its first line to name the site.",
        "Start llms.txt with a line that begins with # and names the site.";
    /// An HTTPS page comes without a `Strict-Transport-Security` header.
    HstsMissing => "hsts-missing", Https, Notice, "No HSTS support",
        "Without it, a browser may still open the first visit over plain HTTP.",
        "Send the Strict-Transport-Security header from the server or the proxy in front of the blog, so browsers always use HTTPS.";
    /// A page links to, or loads, an address on another site that is broken.
    BrokenExternal => "broken-external", OutgoingLinks, Warning, "Page has broken links to other sites",
        "A reader who follows the link finds an error, and the page looks neglected.",
        "Change the link to a page on that site that still exists, or remove it.";
}
