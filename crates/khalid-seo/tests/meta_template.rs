//! Title and description templates with variables (task 1.13, KH-SEO-1).

use khalid_core::{AbsoluteUrl, Locale};
use khalid_seo::{MetaTemplate, PageKind, SeoError, SeoPage, SeoSite, UrlPath};

#[cfg(test)]
fn demo_site() -> SeoSite {
    let mut site = SeoSite::new(
        "Field Notes on SEO",
        AbsoluteUrl::parse("https://www.example.com").unwrap(),
        Locale::try_new("en").unwrap(),
    );
    site.tagline = String::from("Plain advice on search for small teams");
    site
}

#[cfg(test)]
fn post(title: &str) -> SeoPage {
    let mut page = SeoPage::new(
        PageKind::Post,
        Locale::try_new("en").unwrap(),
        UrlPath::try_new("/why-internal-links-matter/").unwrap(),
        title,
    );
    page.excerpt =
        String::from("Internal links help readers and search engines find the rest of your site.");
    page.category = Some(String::from("On-page SEO"));
    page
}

#[cfg(test)]
fn rendered(template: &str, page: &SeoPage) -> String {
    MetaTemplate::parse(template)
        .unwrap()
        .render(&demo_site(), page)
}

#[test]
fn every_variable_takes_its_value_from_the_site_or_the_page() {
    let mut page = post("Why internal links matter");
    page.author = Some(String::from("Asif"));
    assert_eq!(
        rendered(
            "{title} by {author} in {category} {sep} {site}: {tagline}",
            &page
        ),
        "Why internal links matter by Asif in On-page SEO | Field Notes on SEO: \
         Plain advice on search for small teams"
    );
    assert_eq!(rendered("{excerpt}", &page), page.excerpt);
}

#[test]
fn the_separator_is_the_site_setting() {
    let mut site = demo_site();
    site.separator = String::from("·");
    let template = MetaTemplate::parse("{title} {sep} {site}").unwrap();
    assert_eq!(
        template.render(&site, &post("Anchor text")),
        "Anchor text · Field Notes on SEO"
    );
}

#[test]
fn a_missing_value_leaves_no_dangling_separator_or_double_space() {
    let mut page = post("");
    page.category = None;
    assert_eq!(
        rendered("{title} {sep} {category} {sep} {site}", &page),
        "Field Notes on SEO"
    );
    assert_eq!(rendered("  {title}   {sep}  ", &post("Links")), "Links");
}

#[test]
fn a_template_with_an_unknown_variable_is_refused() {
    let error = MetaTemplate::parse("{title} {sep} {sitename}").unwrap_err();
    assert!(
        matches!(&error, SeoError::TemplateVariable { name, .. } if name == "sitename"),
        "{error:?}"
    );
}

#[test]
fn a_template_with_a_lone_brace_is_refused() {
    for bad in [
        "{title {sep} {site}",
        "{title} } {site}",
        "{title",
        "{{title}}",
    ] {
        let error = MetaTemplate::parse(bad).unwrap_err();
        assert!(
            matches!(error, SeoError::TemplateBrace { .. }),
            "{bad:?}: {error:?}"
        );
    }
}

#[test]
fn a_template_keeps_its_source_as_written() {
    let source = "{title} {sep} {category} {sep} {site}";
    assert_eq!(MetaTemplate::parse(source).unwrap().as_str(), source);
    assert_eq!(
        MetaTemplate::parse("About us").unwrap().to_string(),
        "About us"
    );
}
