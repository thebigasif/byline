//! The redirect map: where every old address of a `WordPress` site leads.

use std::fs;
use std::path::Path;

use khalid_core::{Config, Environment};
use khalid_import::wordpress::{
    Export, MapSource, MapTarget, MoveStatus, Plan, REDIRECTS_CSV, Redirection, plan,
    read_redirection, read_wxr,
};
use khalid_routes::Routes;

#[cfg(test)]
fn site(mount: &str) -> Config {
    let toml = format!(
        "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"{mount}\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n"
    );
    Config::from_toml(Path::new("site.toml"), &toml, &Environment::empty()).unwrap()
}

#[cfg(test)]
fn fixture(name: &str) -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    fs::read_to_string(format!("{dir}/fixtures/wordpress/{name}")).unwrap()
}

#[cfg(test)]
fn planned(export: &Export, redirection: Option<&Redirection>, mount: &str) -> Plan {
    plan(export, redirection, &site(mount), &Routes::default())
}

#[cfg(test)]
fn target<'plan>(plan: &'plan Plan, from: &str) -> &'plan MapTarget {
    let entry = plan.redirects.iter().find(|entry| entry.from == from);
    &entry.unwrap_or_else(|| panic!("no entry for {from}")).to
}

#[cfg(test)]
fn moved(to: &str) -> MapTarget {
    MapTarget::Moved {
        to: to.to_owned(),
        status: MoveStatus::Permanent,
    }
}

#[test]
fn every_live_post_page_image_and_category_has_an_entry() {
    let plan = planned(&read_wxr(&fixture("basic.xml")).unwrap(), None, "/blog/");
    let post = "https://northfield.example.com/blog/keep-a-starter-alive/";
    let sources: Vec<(&str, &MapSource)> = plan
        .redirects
        .iter()
        .map(|entry| (entry.from.as_str(), &entry.source))
        .collect();
    assert_eq!(
        sources,
        [
            ("/2024/03/keep-a-starter-alive/", &MapSource::Item(10)),
            ("/about/", &MapSource::Item(11)),
            (
                "/2024/03/keep-a-starter-alive/starter-jar/",
                &MapSource::Item(12)
            ),
            (
                "/category/baking/",
                &MapSource::Category("baking".to_owned())
            ),
            (
                "/category/baking/sourdough/",
                &MapSource::Category("sourdough".to_owned())
            ),
        ]
    );
    assert_eq!(
        target(&plan, "/2024/03/keep-a-starter-alive/"),
        &moved(post)
    );
    assert_eq!(
        target(&plan, "/about/"),
        &moved("https://northfield.example.com/blog/about/")
    );
    assert_eq!(
        target(&plan, "/2024/03/keep-a-starter-alive/starter-jar/"),
        &moved(post),
        "an image's page leads to its post"
    );
    assert_eq!(
        target(&plan, "/category/baking/sourdough/"),
        &moved("https://northfield.example.com/blog/category/sourdough/")
    );
}

#[test]
fn an_address_that_does_not_change_is_listed_as_kept() {
    let text = fixture("basic.xml").replace(
        "https://northfield.example.com/2024/03/keep-a-starter-alive/<",
        "https://northfield.example.com/keep-a-starter-alive/<",
    );
    let plan = planned(&read_wxr(&text).unwrap(), None, "/");
    assert_eq!(target(&plan, "/keep-a-starter-alive/"), &MapTarget::Kept);
    assert!(
        !plan.files[0].text.contains("aliases"),
        "{}",
        plan.files[0].text
    );
}

#[test]
fn every_redirect_is_in_the_csv_file_with_its_status() {
    let plan = planned(&read_wxr(&fixture("basic.xml")).unwrap(), None, "/");
    let csv = plan
        .files
        .iter()
        .find(|file| file.path == Path::new(REDIRECTS_CSV));
    let csv = &csv.unwrap().text;
    assert!(csv.starts_with("from,to,status,match\n"), "{csv}");
    assert!(
        csv.contains("/2024/03/keep-a-starter-alive/,https://northfield.example.com/keep-a-starter-alive/,301,exact\n"),
        "{csv}"
    );
}

#[test]
fn redirection_rules_come_first_and_lead_straight_to_the_new_address() {
    let json = fixture("redirection.json").replace(
        "\"action_data\": { \"url\": \"/recipes/\" }",
        "\"action_data\": { \"url\": \"/2024/03/keep-a-starter-alive/\" }",
    );
    let redirection = read_redirection(&json).unwrap();
    let plan = planned(
        &read_wxr(&fixture("basic.xml")).unwrap(),
        Some(&redirection),
        "/blog/",
    );
    assert_eq!(plan.redirects[0].source, MapSource::Rule(1));
    assert_eq!(
        target(&plan, "/bread-recipes/"),
        &moved("https://northfield.example.com/blog/keep-a-starter-alive/"),
        "no chain through the old post address"
    );
    assert_eq!(target(&plan, "/gluten-free-myths/"), &MapTarget::Gone);
    assert_eq!(target(&plan, "/court-order/"), &MapTarget::Unavailable);
    let pattern = plan
        .redirects
        .iter()
        .find(|entry| entry.source == MapSource::Rule(2));
    assert_eq!(pattern.unwrap().from, r"^/(\d{4})/(\d{2})/(.*)$");
    assert_eq!(
        plan.redirects.len(),
        8 + 5,
        "eight rules and five addresses"
    );
    let csv = &plan.files.last().unwrap().text;
    assert!(csv.contains("\n/gluten-free-myths/,,410,exact\n"), "{csv}");
    assert!(
        csv.contains("\n^/(\\d{4})/(\\d{2})/(.*)$,/blog/$3,301,regex\n"),
        "{csv}"
    );
}

#[test]
fn a_csv_field_with_a_comma_is_quoted() {
    let json = fixture("redirection.json").replace("/summer-sale/", "/sale,summer/");
    let redirection = read_redirection(&json).unwrap();
    let export = read_wxr(&fixture("basic.xml")).unwrap();
    let plan = planned(&export, Some(&redirection), "/blog/");
    let csv = &plan.files.last().unwrap().text;
    assert!(
        csv.contains("\n\"/sale,summer/\",/offers/,302,exact\n"),
        "{csv}"
    );
}
