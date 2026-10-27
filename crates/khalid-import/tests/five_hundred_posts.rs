//! The done line of task 6.1: "a 500-post `WordPress` import loses nothing".
//! Zero lost posts, every slug kept, and a redirect map that covers every
//! old address. The export is generated here, the same on every run.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use khalid_content::{ContentTree, read_content};
use khalid_core::{Config, Environment};
use khalid_import::wordpress::{MapSource, Place, Plan, plan, read_redirection, read_wxr};
use khalid_import::{MapTarget, SlugOrigin};
use khalid_routes::Routes;

const POSTS: u64 = 500;
const OLD: &str = "https://northfield.example.com";
const WORDS: [&str; 8] = [
    "rye-loaf",
    "starter",
    "spelt",
    "crumb",
    "proofing",
    "flour",
    "oven-spring",
    "scoring",
];
const CATEGORIES: [(&str, &str, &str); 4] = [
    ("baking", "Baking", ""),
    ("sourdough", "Sourdough &amp; starters", "baking"),
    ("rye", "Rye", ""),
    ("pastry", "Pastry", ""),
];
const AUTHORS: [(&str, &str); 3] = [
    ("mara", "Mara Lind"),
    ("tom.b", "Tom Becker"),
    ("imoreau", "Inès Moreau"),
];

#[cfg(test)]
fn slug(n: u64) -> String {
    format!("{}-{n}", WORDS[usize::try_from(n).unwrap() % WORDS.len()])
}

#[cfg(test)]
fn old_path(n: u64) -> String {
    format!("/{}/{:02}/{}/", 2015 + n % 10, 1 + n % 12, slug(n))
}

#[cfg(test)]
fn meta(key: &str, value: &str) -> String {
    format!(
        "<wp:postmeta><wp:meta_key><![CDATA[{key}]]></wp:meta_key><wp:meta_value><![CDATA[{value}]]></wp:meta_value></wp:postmeta>\n"
    )
}

/// One post, with a body, SEO fields from one of the four plugins, and an
/// image of its own for every fifth post.
#[cfg(test)]
fn post(n: u64, items: &mut String) {
    let i = usize::try_from(n).unwrap();
    let (category, _, _) = CATEGORIES[i % 4];
    let (login, _) = AUTHORS[i % 3];
    let date = format!(
        "{}-{:02}-{:02} 09:30:00",
        2015 + n % 10,
        1 + n % 12,
        1 + n % 28
    );
    let body = match n % 3 {
        0 => format!(
            "<!-- wp:paragraph -->\n<p>Loaf {n} needs <strong>time</strong>.</p>\n<!-- /wp:paragraph -->"
        ),
        1 => format!(
            "Loaf {n} needs time.\n\nThen [caption id=\"{n}\"]a crumb[/caption] more.\n::not a block"
        ),
        _ => format!(
            "<p>Watch loaf {n}.</p><iframe src=\"https://www.youtube.com/embed/abc{n}\" title=\"Loaf {n}\"></iframe>"
        ),
    };
    let noindex = if n.is_multiple_of(40) { "1" } else { "0" };
    let seo = match n % 4 {
        0 => [
            meta("_yoast_wpseo_title", "%%title%% %%sep%% %%sitename%%"),
            meta("_yoast_wpseo_meta-robots-noindex", noindex),
        ],
        1 => [
            meta("rank_math_description", "%title% from the bakery"),
            meta("rank_math_robots", "a:1:{i:0;s:8:\"nofollow\";}"),
        ],
        2 => [
            meta("_aioseo_description", "#post_title, baked slowly"),
            String::new(),
        ],
        _ => [
            meta(
                "_seopress_robots_canonical",
                "https://example.org/original/",
            ),
            meta("_seopress_analysis_target_kw", "bread,rye"),
        ],
    }
    .concat();
    let image = if n.is_multiple_of(5) {
        meta("_thumbnail_id", &(5000 + n).to_string())
    } else {
        String::new()
    };
    let _ = write!(
        items,
        "<item><title><![CDATA[Bread &amp; loaf {n}]]></title><link>{OLD}{}</link>\n\
         <dc:creator><![CDATA[{login}]]></dc:creator>\n\
         <content:encoded><![CDATA[{body}]]></content:encoded><excerpt:encoded><![CDATA[]]></excerpt:encoded>\n\
         <wp:post_id>{}</wp:post_id><wp:post_date><![CDATA[{date}]]></wp:post_date><wp:post_name><![CDATA[{}]]></wp:post_name>\n\
         <wp:status><![CDATA[publish]]></wp:status><wp:post_type><![CDATA[post]]></wp:post_type><wp:is_sticky>{}</wp:is_sticky>\n\
         <category domain=\"category\" nicename=\"{category}\"><![CDATA[x]]></category><category domain=\"post_tag\" nicename=\"tag-{}\"><![CDATA[x]]></category>\n\
         {seo}{image}</item>\n",
        old_path(n),
        1000 + n,
        slug(n),
        u8::from(n.is_multiple_of(50)),
        n % 7,
    );
    if n.is_multiple_of(5) {
        let _ = write!(
            items,
            "<item><title>photo</title><link>{OLD}{}photo-{n}/</link><wp:post_id>{}</wp:post_id>\n\
             <wp:post_parent>{}</wp:post_parent><wp:status>inherit</wp:status><wp:post_type>attachment</wp:post_type>\n\
             <wp:attachment_url>{OLD}/wp-content/uploads/photo-{n}.jpg</wp:attachment_url></item>\n",
            old_path(n),
            5000 + n,
            1000 + n,
        );
    }
}

#[cfg(test)]
fn export() -> String {
    let mut text = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<rss version=\"2.0\" xmlns:wp=\"http://wordpress.org/export/1.2/\" xmlns:excerpt=\"http://wordpress.org/export/1.2/excerpt/\" xmlns:content=\"http://purl.org/rss/1.0/modules/content/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n<channel><title>Northfield Bakery Blog</title><link>https://northfield.example.com</link><wp:wxr_version>1.2</wp:wxr_version>\n",
    );
    for (login, name) in AUTHORS {
        let _ = writeln!(
            text,
            "<wp:author><wp:author_login><![CDATA[{login}]]></wp:author_login><wp:author_display_name><![CDATA[{name}]]></wp:author_display_name></wp:author>"
        );
    }
    for (id, (slug, name, parent)) in (1_u64..).zip(CATEGORIES) {
        let _ = writeln!(
            text,
            "<wp:category><wp:term_id>{id}</wp:term_id><wp:category_nicename><![CDATA[{slug}]]></wp:category_nicename><wp:category_parent><![CDATA[{parent}]]></wp:category_parent><wp:cat_name><![CDATA[{name}]]></wp:cat_name></wp:category>"
        );
    }
    for n in 0..POSTS {
        post(n, &mut text);
    }
    text.push_str("</channel></rss>\n");
    text
}

#[cfg(test)]
fn planned(mount: &str) -> Plan {
    let toml = format!(
        "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"{mount}\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n"
    );
    let site = Config::from_toml(Path::new("site.toml"), &toml, &Environment::empty()).unwrap();
    let rules = format!(
        r#"{{"redirects":[{{"id":1,"url":"/best-loaf/","action_code":301,"action_type":"url","action_data":{{"url":"{OLD}{}"}},"match_type":"url","regex":false,"enabled":true}}]}}"#,
        old_path(0)
    );
    let redirection = read_redirection(&rules).unwrap();
    plan(
        &read_wxr(&export()).unwrap(),
        Some(&redirection),
        &site,
        &Routes::default(),
    )
}

#[test]
fn no_post_is_lost_and_every_slug_is_kept() {
    let plan = planned("/blog/");
    assert_eq!(plan.failed.len(), 0, "{:?}", plan.failed);
    let posts: Vec<_> = plan
        .items
        .iter()
        .filter(|item| item.place == Place::Post)
        .collect();
    let ids: BTreeSet<u64> = posts.iter().map(|item| item.id).collect();
    assert_eq!(ids, (1000..1000 + POSTS).collect::<BTreeSet<u64>>());
    for (n, item) in (0..POSTS).zip(&posts) {
        assert_eq!(item.slug_origin, SlugOrigin::Kept, "post {n}");
        assert_eq!(item.slug.as_str(), slug(n), "post {n}");
    }
}

#[test]
fn every_old_address_has_one_entry_that_leads_to_a_page_khalid_has() {
    let plan = planned("/blog/");
    let mut targets: BTreeSet<String> = plan
        .items
        .iter()
        .filter_map(|item| {
            item.address
                .as_ref()
                .map(|address| format!("{OLD}{address}"))
        })
        .collect();
    targets.extend(
        ["baking", "sourdough", "rye", "pastry"].map(|c| format!("{OLD}/blog/category/{c}/")),
    );
    let old_paths: Vec<&str> = plan
        .redirects
        .iter()
        .map(|entry| entry.from.as_str())
        .collect();
    let unique: BTreeSet<&str> = old_paths.iter().copied().collect();
    assert_eq!(unique.len(), old_paths.len(), "each old address once");
    assert_eq!(
        old_paths.len(),
        1 + 500 + 100 + 4,
        "a rule, the posts, their images and the categories"
    );
    for n in 0..POSTS {
        assert!(
            unique.contains(old_path(n).as_str()),
            "post {n} is in the map"
        );
    }
    for entry in &plan.redirects {
        let MapTarget::Moved { to, .. } = &entry.to else {
            panic!("{entry:?} should move");
        };
        assert!(
            targets.contains(to),
            "{} leads to {to}, which Khalid does not answer",
            entry.from
        );
    }
    assert_eq!(plan.redirects[0].source, MapSource::Rule(1));
    assert_eq!(
        plan.redirects[0].to, plan.redirects[1].to,
        "the rule leads straight to post 0"
    );
}

#[test]
fn the_written_site_reads_back_with_every_post() {
    let site = tempfile::tempdir().unwrap();
    let plan = planned("/");
    plan.write(site.path()).unwrap();
    let content = read_content(&ContentTree::walk(site.path()).unwrap()).unwrap();
    assert_eq!(content.posts.len(), 500);
    assert_eq!(content.authors.len(), 3);
    let slugs: BTreeSet<String> = content
        .posts
        .iter()
        .filter_map(|post| post.front.slug.as_ref().map(ToString::to_string))
        .collect();
    assert_eq!(slugs, (0..POSTS).map(slug).collect::<BTreeSet<String>>());
    for post in &content.posts {
        let slug = post.front.slug.as_ref().unwrap();
        assert_eq!(
            post.front.aliases.len(),
            1,
            "{slug}: the old dated address is an alias"
        );
    }
    let report = plan.report().to_string();
    assert!(
        report.contains("Items left behind: an uploaded file whose page leads to its post (100: 5000, 5005, 5010 and 97 more).\n"),
        "{report}"
    );
}
