//! The redirect map: where every old address of a `WordPress` site leads.

use std::fs;
use std::path::Path;

use khalid_core::{Config, Environment};
use khalid_import::wordpress::{Export, MapSource, MapTarget, MoveStatus, Plan, plan, read_wxr};
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
fn planned(export: &Export, mount: &str) -> Plan {
    plan(export, &site(mount), &Routes::default())
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
    let plan = planned(&read_wxr(&fixture("basic.xml")).unwrap(), "/blog/");
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
    let plan = planned(&read_wxr(&text).unwrap(), "/");
    assert_eq!(target(&plan, "/keep-a-starter-alive/"), &MapTarget::Kept);
    assert!(
        !plan.files[0].text.contains("aliases"),
        "{}",
        plan.files[0].text
    );
}
