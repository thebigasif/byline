//! Turning a Substack export into Khalid content files.

use std::path::Path;

use khalid_core::{AbsoluteUrl, Config, Environment};
use khalid_import::substack::{LeftReason, MapEntry, Plan, REDIRECTS_CSV, plan, read_export};
use khalid_import::{MapTarget, MoveStatus, Place, SlugOrigin};
use khalid_routes::Routes;

const POSTS_CSV: &str = "post_id,post_date,is_published,type,audience,title,subtitle\n\
    140062712.why-rye,2024-03-12T09:30:44.123Z,true,newsletter,everyone,Why rye,A grain that waits\n\
    140062713.members-loaf,2024-04-02T08:00:00.000Z,true,newsletter,only_paid,The members loaf,\n\
    140062714.spring-flour,,false,newsletter,everyone,Spring flour,\n\
    140062715.ask-me,2024-05-01T10:00:00.000Z,true,thread,everyone,Ask me anything,\n\
    140062716.about,2024-01-01T10:00:00.000Z,true,page,everyone,About,\n\
    140062717.episode-one,2024-06-01T10:00:00.000Z,true,podcast,everyone,Episode one,\n";

const FILES: [(&str, &str); 5] = [
    (
        "140062712.why-rye.html",
        "<p>Rye needs time.</p><div class=\"subscription-widget-wrap\"></div>",
    ),
    ("140062713.members-loaf.html", "<p>For members.</p>"),
    ("140062714.spring-flour.html", "<p>Soon.</p>"),
    ("140062715.ask-me.html", "<p>Ask.</p>"),
    ("140062716.about.html", "<p>We bake.</p>"),
];

/// The plan of the export from the Substack at `old` into a blog mounted
/// at `/blog/` on northfield.example.com.
#[cfg(test)]
fn planned(old: &str) -> Plan {
    let toml = "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"/blog/\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    let site = Config::from_toml(Path::new("site.toml"), toml, &Environment::empty()).unwrap();
    let export = read_export(POSTS_CSV, FILES).unwrap();
    plan(
        &export,
        &AbsoluteUrl::parse(old).unwrap(),
        &site,
        &Routes::default(),
    )
}

#[test]
fn a_published_post_keeps_its_slug_date_and_subtitle() {
    let plan = planned("https://rye.substack.com");
    let file = &plan.files[0];
    assert_eq!(
        file.path,
        Path::new("content/en/posts/2024-03-12-why-rye.md")
    );
    assert_eq!(
        file.text,
        "---\ntitle: \"Why rye\"\nslug: \"why-rye\"\ndate: \"2024-03-12\"\ndescription: \"A grain that waits\"\n---\n\nRye needs time.\n"
    );
    let post = &plan.posts[0];
    assert_eq!(
        (post.slug.as_str(), &post.slug_origin),
        ("why-rye", &SlugOrigin::Kept)
    );
    assert_eq!(post.address.as_deref(), Some("/blog/why-rye/"));
    let old = post.old.as_ref().map(AbsoluteUrl::as_str);
    assert_eq!(old, Some("https://rye.substack.com/p/why-rye"));
    assert_eq!(post.left, ["subscribe box"]);
}

#[test]
fn paid_posts_are_for_members_and_drafts_and_pages_go_to_their_folders() {
    let plan = planned("https://rye.substack.com");
    let places: Vec<(&str, Place, bool)> = plan
        .posts
        .iter()
        .map(|post| (post.slug.as_str(), post.place, post.members))
        .collect();
    assert_eq!(
        places,
        [
            ("why-rye", Place::Post, false),
            ("members-loaf", Place::Post, true),
            ("spring-flour", Place::Draft, false),
            ("about", Place::Page, false),
        ]
    );
    assert!(
        plan.files[1].text.contains("access: \"members\"\n"),
        "{}",
        plan.files[1].text
    );
    assert_eq!(plan.files[2].path, Path::new("drafts/en/spring-flour.md"));
    assert_eq!(plan.files[3].path, Path::new("content/en/pages/about.md"));
}

#[test]
fn threads_and_posts_without_text_stay_behind() {
    let plan = planned("https://rye.substack.com");
    let behind: Vec<(&str, &LeftReason)> = plan
        .left_behind
        .iter()
        .map(|post| (post.id.as_str(), &post.reason))
        .collect();
    assert_eq!(
        behind,
        [
            ("140062715", &LeftReason::Thread),
            ("140062717", &LeftReason::NoBody)
        ]
    );
    assert!(plan.failed.is_empty(), "{:?}", plan.failed);
}

#[test]
fn an_old_address_inside_the_new_blog_becomes_an_alias() {
    let plan = planned("https://northfield.example.com/blog/");
    let text = &plan.files[0].text;
    assert!(
        text.ends_with("aliases:\n  - \"/blog/p/why-rye\"\n---\n\nRye needs time.\n"),
        "{text}"
    );
    let old = plan.posts[0].old.as_ref().map(AbsoluteUrl::as_str);
    assert_eq!(old, Some("https://northfield.example.com/blog/p/why-rye"));
}

#[test]
fn every_planned_file_is_written_and_none_twice() {
    let plan = planned("https://rye.substack.com");
    let site = tempfile::tempdir().unwrap();
    let written = plan.write(site.path()).unwrap();
    assert_eq!(written.len(), plan.files.len());
    assert!(plan.write(site.path()).is_err());
}

#[test]
fn every_published_post_maps_its_old_address_and_the_csv_holds_the_moves() {
    let plan = planned("https://rye.substack.com");
    let map: Vec<(&str, &MapTarget)> = plan
        .redirects
        .iter()
        .map(|entry| (entry.from.as_str(), &entry.to))
        .collect();
    let to = |path: &str| MapTarget::Moved {
        to: format!("https://northfield.example.com{path}"),
        status: MoveStatus::Permanent,
    };
    assert_eq!(
        map,
        [
            ("https://rye.substack.com/p/why-rye", &to("/blog/why-rye/")),
            (
                "https://rye.substack.com/p/members-loaf",
                &to("/blog/members-loaf/")
            ),
            (
                "https://rye.substack.com/p/about",
                &to("/blog/pages/about/")
            ),
        ]
    );
    assert!(plan.redirects.iter().all(MapEntry::on_substack));
    let csv = plan.files.last().unwrap();
    assert_eq!(csv.path, Path::new(REDIRECTS_CSV));
    assert!(
        csv.text.starts_with("from,to,status,match\nhttps://rye.substack.com/p/why-rye,https://northfield.example.com/blog/why-rye/,301,exact\n"),
        "{}",
        csv.text
    );
}

#[test]
fn an_old_address_on_the_new_host_is_written_as_a_path() {
    let plan = planned("https://northfield.example.com");
    let csv = &plan.files.last().unwrap().text;
    assert!(
        csv.contains("\n/p/why-rye,https://northfield.example.com/blog/why-rye/,301,exact\n"),
        "{csv}"
    );
}

#[test]
fn the_report_counts_what_comes_in_and_names_what_stays_behind() {
    let report = planned("https://rye.substack.com").report().to_string();
    assert_eq!(
        report,
        "This is a dry run: nothing was written.\n\
         Coming in: 2 posts, 1 page and 1 draft.\n\
         For signed-in members only, as on Substack: 1.\n\
         Slugs kept: 4.\n\
         Old addresses mapped: 3, of which 3 move and 0 stay the same.\n\
         Old addresses on substack.com: 3. Substack cannot redirect them, so link each post there to its new home.\n\
         Content with no place in Khalid, left out: subscribe box (1 post).\n\
         Posts left behind: 140062715, a discussion thread, which lives in Substack's comments; 140062717, the export has no text for it.\n\
         Posts that could not come in: 0.\n\
         Rows of posts.csv that could not be read: 0.\n"
    );
}
