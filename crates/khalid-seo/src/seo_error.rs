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
    /// A redirect leads to an address that is neither a path nor a valid
    /// `http` or `https` address.
    #[error("the redirect destination {address:?} is not a path or a web address")]
    RedirectAddress {
        /// The destination, as given.
        address: String,
        /// Why it was refused.
        #[source]
        source: ParseUrlError,
    },
    /// A redirect's status and destination do not fit together.
    #[error("the redirect from {from:?} {problem}")]
    RedirectColumns {
        /// The path or pattern the redirect starts from.
        from: String,
        /// What is wrong with it.
        problem: &'static str,
    },
    /// A row of a redirect list has the wrong shape.
    #[error("line {line} of the redirect list {problem}")]
    RedirectCsvRow {
        /// The line the row starts on, counting from 1.
        line: usize,
        /// What is wrong with it.
        problem: &'static str,
    },
    /// A row of a redirect list holds a path, address or pattern that is
    /// refused.
    #[error("line {line} of the redirect list holds a value that is refused")]
    RedirectCsvValue {
        /// The line the row starts on, counting from 1.
        line: usize,
        /// Why the value was refused.
        #[source]
        source: Box<SeoError>,
    },
    /// A domain in the link policy is not a plain domain name.
    #[error("{domain:?} is not a plain domain name such as example.com")]
    Domain {
        /// The domain, as given.
        domain: String,
    },
    /// A currency is not a three-letter ISO 4217 code such as `GBP`.
    #[error("{currency:?} is not a currency code such as GBP or EUR")]
    Currency {
        /// The code, as given.
        currency: String,
    },
    /// A rating is below zero, above its best score, or its best is not above zero.
    #[error("a rating of {value} out of {best} is not between 0 and its best score")]
    Rating {
        /// The score, as given.
        value: f64,
        /// The best possible score, as given.
        best: f64,
    },
    /// A schema type name is not one capitalised word such as `Course`.
    #[error("{name:?} is not a schema.org type name such as Course")]
    SchemaType {
        /// The name, as given.
        name: String,
    },
    /// A schema property name is not one word starting small, such as `provider`.
    #[error("{name:?} is not a schema.org property name such as provider")]
    SchemaProperty {
        /// The name, as given.
        name: String,
    },
    /// A custom JSON-LD field is not valid JSON.
    #[error("the custom JSON-LD is not valid JSON at line {line}, column {column}")]
    CustomSchemaSyntax {
        /// The line of the first mistake, counting from 1.
        line: usize,
        /// The column of the first mistake, counting from 1.
        column: usize,
    },
    /// A custom JSON-LD field is JSON, but not JSON-LD this crate can write.
    #[error("in the custom JSON-LD, {} {problem}", if at.is_empty() { "the field" } else { at })]
    CustomSchema {
        /// Where the problem is, such as `location.@type`. Empty for the whole field.
        at: String,
        /// What is wrong there.
        problem: &'static str,
    },
    /// A changed address is outside the folder that holds the `IndexNow` key
    /// file, so search engines would refuse the request.
    #[error("the address {url} is outside {scope}, where the IndexNow key file applies")]
    IndexNowScope {
        /// The address, as given.
        url: String,
        /// The folder the key file applies to, such as `https://www.example.com/blog/`.
        scope: String,
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
