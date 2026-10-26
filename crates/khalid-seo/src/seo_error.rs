//! The one error type of this crate (BL-ERR-1).

use khalid_core::ParseUrlError;

/// Something in the SEO input that cannot become valid output.
///
/// The binary turns each variant into a message with what, why and fix
/// (BL-ERR-4). The text here names the problem only.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum SeoError {
    /// A title or description template names a variable that does not exist.
    #[error("the template {template:?} uses {{{name}}}, which is not a known variable")]
    TemplateVariable {
        /// The whole template, as written.
        template: String,
        /// The unknown name between the braces.
        name: String,
    },
    /// A title or description template has a `{` without its `}`, or the reverse.
    #[error("the template {template:?} has a brace without its partner")]
    TemplateBrace {
        /// The whole template, as written.
        template: String,
    },
    /// A path is not a clean absolute path such as `/blog/why-links-matter/`.
    #[error("the path {path:?} {problem}")]
    Path {
        /// The path, as given.
        path: String,
        /// What is wrong with it.
        problem: &'static str,
    },
    /// A Twitter handle is empty, too long, or holds a character Twitter refuses.
    #[error("the Twitter handle {handle:?} is not a valid account name")]
    TwitterHandle {
        /// The handle, as given.
        handle: String,
    },
    /// A site verification code holds a character no search engine uses.
    #[error("the verification code {code:?} holds a character that is not allowed")]
    VerificationCode {
        /// The code, as given.
        code: String,
    },
    /// A verification file name is not a plain file name at the root.
    #[error("the verification file name {name:?} {problem}")]
    RootFile {
        /// The file name, as given.
        name: String,
        /// What is wrong with it.
        problem: &'static str,
    },
    /// A translation key is empty.
    #[error("the translation key {key:?} is empty")]
    TranslationKey {
        /// The key, as given.
        key: String,
    },
    /// Two pages with the same translation key are in the same language.
    #[error("two pages with the translation key {key:?} are both in {language}")]
    TranslationLanguage {
        /// The shared translation key.
        key: String,
        /// The language both pages are in.
        language: String,
    },
    /// A crawler name in the robots policy holds a character robots.txt refuses.
    #[error("the crawler name {agent:?} may hold only letters, digits, - and _")]
    UserAgent {
        /// The name, as given.
        agent: String,
    },
    /// A robots.txt path rule does not start with `/` or holds a space.
    #[error("the robots.txt rule {path:?} must start with / and hold no spaces")]
    RobotsPath {
        /// The rule, as given.
        path: String,
    },
    /// Two redirect rules start from the same path.
    #[error("the path {from:?} has more than one redirect rule")]
    RedirectDuplicate {
        /// The path both rules start from.
        from: String,
    },
    /// Following redirects from a path comes back round to it.
    #[error("the redirects from {from:?} go round in a loop")]
    RedirectLoop {
        /// The first path found in the loop.
        from: String,
    },
    /// A redirect pattern is not a valid regular expression.
    #[error("the redirect pattern {pattern:?} is not a valid regular expression")]
    RedirectPattern {
        /// The pattern, as given.
        pattern: String,
        /// Why it was refused.
        #[source]
        source: regex::Error,
    },
    /// A pattern's destination is neither a path nor a web address.
    #[error("the redirect destination {template:?} must start with / or https://")]
    RedirectTemplate {
        /// The destination, as given.
        template: String,
    },
    /// A domain in the link policy is not a plain domain name.
    #[error("{domain:?} is not a plain domain name such as example.com")]
    Domain {
        /// The domain, as given.
        domain: String,
    },
    /// The address built for a page is not a valid absolute URL.
    #[error("the address built for {path:?} is not a valid URL")]
    Url {
        /// The page path the address was built for.
        path: String,
        /// Why the address was refused.
        #[source]
        source: ParseUrlError,
    },
}
