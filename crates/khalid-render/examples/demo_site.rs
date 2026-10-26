//! Writes every page type of the demo site, in English and German, to a
//! folder, with the real stylesheet, so the pages can be opened in a browser.
//!
//! ```text
//! cargo run -p khalid-render --example demo_site -- target/demo-site
//! python3 -m http.server 8000 --directory target/demo-site
//! ```
//!
//! Then open <http://localhost:8000/blog/>. Content files are read with a
//! deliberately small reader, enough for the demo's plain markdown. The real
//! reader is `khalid-content`, which task 1.15 wires in.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use khalid_core::{AbsoluteUrl, Locale};
use khalid_render::{
    Alternate, BodyHtml, Features, GlossaryGroup, GlossaryTerm, Link, Nav, NavLink, PageBody,
    PageKind, PageMeta, Person, PostCard, RenderAuthor, RenderGlossary, RenderListing, RenderPage,
    RenderPost, RenderSearch, RenderSite, Renderer, Robots, Tokens, stylesheet,
};

type Fallible<T> = Result<T, Box<dyn Error>>;

const ADDRESS: &str = "https://www.example.com";

/// One language of the demo site and the words its menus use.
struct Lang {
    tag: &'static str,
    prefix: &'static str,
    name: &'static str,
    glossary: &'static str,
    search: &'static str,
}

const LANGUAGES: [Lang; 2] = [
    Lang {
        tag: "en",
        prefix: "/blog/",
        name: "English",
        glossary: "Glossary",
        search: "Search",
    },
    Lang {
        tag: "de",
        prefix: "/blog/de/",
        name: "Deutsch",
        glossary: "Glossar",
        search: "Suche",
    },
];

/// A content file: its front matter as text fields, and its markdown body.
struct Doc {
    fields: BTreeMap<String, String>,
    body: String,
}

impl Doc {
    fn field(&self, key: &str) -> String {
        self.fields.get(key).cloned().unwrap_or_default()
    }
}

/// Every markdown file in `folder` but `_index.md`, by name.
fn docs(folder: &Path) -> Fallible<Vec<Doc>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(folder)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .filter(|path| !path.ends_with("_index.md"))
        .collect();
    paths.sort();
    paths.iter().map(|path| read_doc(path)).collect()
}

fn read_doc(path: &Path) -> Fallible<Doc> {
    let text = fs::read_to_string(path)?;
    let rest = text.strip_prefix("---\n").ok_or("no front matter")?;
    let (front, body) = rest
        .split_once("\n---\n")
        .ok_or("front matter not closed")?;
    let fields = front
        .lines()
        .filter_map(|line| line.split_once(": "))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
    Ok(Doc {
        fields,
        body: body.to_owned(),
    })
}

/// Inline markdown: `code`, `[[slug]]` glossary links and `[text](href)`.
fn inline(text: &str, glossary: &str) -> String {
    let text = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let mut html = String::new();
    let mut rest = text.as_str();
    while let Some((before, after)) = rest.split_once('[') {
        html.push_str(before);
        if let Some(inner) = after.strip_prefix('[')
            && let Some((slug, after)) = inner.split_once("]]")
        {
            let _ = write!(
                html,
                r#"<a href="{glossary}{slug}/">{}</a>"#,
                slug.replace('-', " ")
            );
            rest = after;
        } else if let Some((label, after)) = after.split_once("](")
            && let Some((href, after)) = after.split_once(')')
        {
            let _ = write!(html, r#"<a href="{href}">{label}</a>"#);
            rest = after;
        } else {
            html.push('[');
            rest = after;
        }
    }
    html.push_str(rest);
    let mut out = String::new();
    for (index, part) in html.split('`').enumerate() {
        if index > 0 {
            out.push_str(if index % 2 == 1 { "<code>" } else { "</code>" });
        }
        out.push_str(part);
    }
    out
}

/// Block markdown: `##` headings, `-` lists and paragraphs.
fn markdown(text: &str, glossary: &str) -> BodyHtml {
    let mut html = String::new();
    for block in text
        .split("\n\n")
        .map(str::trim)
        .filter(|block| !block.is_empty())
    {
        if let Some(heading) = block.strip_prefix("## ") {
            let _ = writeln!(html, "<h2>{}</h2>", inline(heading, glossary));
        } else if block.starts_with("- ") {
            html.push_str("<ul>\n");
            for item in block.lines().filter_map(|line| line.strip_prefix("- ")) {
                let _ = writeln!(html, "<li>{}</li>", inline(item, glossary));
            }
            html.push_str("</ul>\n");
        } else {
            let _ = writeln!(
                html,
                "<p>{}</p>",
                inline(&block.replace('\n', " "), glossary)
            );
        }
    }
    BodyHtml::from_sanitised(html)
}

/// The demo being written: where to, with what, and how many pages so far.
struct Demo {
    out: PathBuf,
    fixture: PathBuf,
    renderer: Renderer,
    stylesheet: String,
    pages: usize,
    /// Each `translation` key's path in every language that has it.
    translations: BTreeMap<String, BTreeMap<&'static str, String>>,
    /// Category names by `language/slug`, from `categories.toml`.
    categories: BTreeMap<String, String>,
}

/// The category names in every language, from `categories.toml`.
fn category_names(fixture: &Path) -> Fallible<BTreeMap<String, String>> {
    let mut path = fixture.to_path_buf();
    path.push("categories.toml");
    let table: toml::Table = toml::from_str(&fs::read_to_string(path)?)?;
    let list = table.get("categories").and_then(toml::Value::as_array);
    let mut names = BTreeMap::new();
    for entry in list.ok_or("categories.toml has no categories")? {
        let slug = entry
            .get("slug")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        for lang in &LANGUAGES {
            if let Some(name) = entry.get(lang.tag).and_then(toml::Value::as_str) {
                names.insert(format!("{}/{slug}", lang.tag), name.to_owned());
            }
        }
    }
    Ok(names)
}

impl Demo {
    fn site(&self, lang: &Lang) -> Fallible<RenderSite> {
        let link = |label: &str, href: String| NavLink {
            label: label.to_owned(),
            href,
        };
        Ok(RenderSite {
            name: "Field Notes on SEO".to_owned(),
            locale: Locale::try_new(lang.tag)?,
            home: lang.prefix.to_owned(),
            stylesheet: self.stylesheet.clone(),
            nav: Nav {
                primary: vec![
                    link(lang.glossary, format!("{}glossary/", lang.prefix)),
                    link(lang.search, format!("{}search/", lang.prefix)),
                ],
                footer: Vec::new(),
            },
            main_site: Some(link("example.com", format!("{ADDRESS}/"))),
            share_image: None,
            features: Features::all(),
        })
    }

    /// This page's path in every language: from its `translation` key when it
    /// has one, otherwise the same path under each language's prefix.
    fn twins(&self, lang: &Lang, path: &str, key: Option<&str>) -> Fallible<Vec<(&Lang, String)>> {
        if let Some(paths) = key.and_then(|key| self.translations.get(key)) {
            return Ok(LANGUAGES
                .iter()
                .filter_map(|other| Some((other, paths.get(other.tag)?.clone())))
                .collect());
        }
        let rest = path
            .strip_prefix(lang.prefix)
            .ok_or("path outside the language")?;
        Ok(LANGUAGES
            .iter()
            .map(|other| (other, format!("{}{rest}", other.prefix)))
            .collect())
    }

    /// Renders `body` at `path` in `lang`, with its other languages as
    /// alternates, and writes it.
    fn page(
        &mut self,
        lang: &Lang,
        (path, key): (&str, Option<&str>),
        title: &str,
        body: &PageBody,
    ) -> Fallible<()> {
        let alternates = self
            .twins(lang, path, key)?
            .into_iter()
            .map(|(other, twin)| {
                Ok(Alternate {
                    locale: Locale::try_new(other.tag)?,
                    href: AbsoluteUrl::parse(&format!("{ADDRESS}{twin}"))?,
                    label: other.name.to_owned(),
                })
            })
            .collect::<Fallible<Vec<_>>>()?;
        let kind = if matches!(body, PageBody::Post(_)) {
            PageKind::Article
        } else {
            PageKind::Website
        };
        let robots = if matches!(body, PageBody::NotFound) {
            Robots::NoIndex
        } else {
            Robots::Index
        };
        let meta = PageMeta {
            title: title.to_owned(),
            description: format!("{title}, from Field Notes on SEO."),
            canonical: AbsoluteUrl::parse(&format!("{ADDRESS}{path}"))?,
            path: path.to_owned(),
            kind,
            robots,
            image: None,
            x_default: alternates.first().map(|first| first.href.clone()),
            seo_head: None,
            alternates,
        };
        let html = self.renderer.render(&self.site(lang)?, &meta, body)?;
        let mut file = self.out.clone();
        file.push(path.trim_start_matches('/'));
        if path.ends_with('/') {
            file.push("index.html");
        }
        fs::create_dir_all(file.parent().ok_or("no folder")?)?;
        fs::write(file, html)?;
        self.pages += 1;
        Ok(())
    }

    fn content(&self, lang: &Lang, folder: &str) -> Fallible<Vec<Doc>> {
        let mut path = self.fixture.clone();
        path.push(format!("content/{}/{folder}", lang.tag));
        docs(&path)
    }

    /// Every page of one language.
    fn language(&mut self, lang: &Lang) -> Fallible<()> {
        let authors = self.content(lang, "authors")?;
        let mut posts = self.content(lang, "posts")?;
        posts.sort_by_key(|post| std::cmp::Reverse(post.field("date")));
        let mut cards = Vec::new();
        for post in &posts {
            let card = self.post(lang, post, &authors)?;
            cards.push((post.field("author"), post.field("category"), card));
        }
        let pick = |title: &str, keep: &dyn Fn(&str, &str) -> bool| RenderListing {
            title: title.to_owned(),
            intro: None,
            posts: cards
                .iter()
                .filter(|(author, category, _)| keep(author, category))
                .map(|(_, _, card)| card.clone())
                .collect(),
            pagination: None,
        };
        let home = PageBody::Home(pick("Field Notes on SEO", &|_, _| true));
        self.page(lang, (lang.prefix, None), "Field Notes on SEO", &home)?;
        for author in &authors {
            let slug = author.field("slug");
            let page = RenderAuthor {
                name: author.field("name"),
                role: Some(author.field("role")),
                image: None,
                bio: markdown(&author.body, &format!("{}glossary/", lang.prefix)),
                profiles: Vec::new(),
                posts: pick("", &|by, _| by == slug).posts,
                pagination: None,
            };
            let (path, name) = (format!("{}authors/{slug}/", lang.prefix), page.name.clone());
            self.page(
                lang,
                (&path, None),
                &name,
                &PageBody::Author(Box::new(page)),
            )?;
        }
        let used: BTreeSet<String> = cards
            .iter()
            .map(|(_, category, _)| category.clone())
            .collect();
        for slug in used {
            let name = self.category_name(lang, &slug);
            let path = format!("{}category/{slug}/", lang.prefix);
            let body = PageBody::Category(pick(&name, &|_, category| category == slug));
            self.page(lang, (&path, None), &name, &body)?;
        }
        self.glossary_and_search(lang)
    }

    fn category_name(&self, lang: &Lang, slug: &str) -> String {
        let key = format!("{}/{slug}", lang.tag);
        self.categories
            .get(&key)
            .cloned()
            .unwrap_or_else(|| slug.to_owned())
    }

    /// The glossary from A to Z, a page per term, the search page and the 404 page.
    fn glossary_and_search(&mut self, lang: &Lang) -> Fallible<()> {
        let glossary = format!("{}glossary/", lang.prefix);
        let mut terms = Vec::new();
        for term in self.content(lang, "glossary")? {
            let path = format!("{glossary}{}/", term.field("slug"));
            let page = RenderPage {
                title: term.field("term"),
                updated: None,
                body: markdown(&term.body, &glossary),
            };
            let key = term.field("translation");
            self.page(
                lang,
                (&path, Some(&key)),
                &term.field("term"),
                &PageBody::Page(page),
            )?;
            let first = term
                .body
                .split(". ")
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned();
            terms.push(GlossaryTerm {
                term: term.field("term"),
                href: path,
                summary: format!("{first}."),
            });
        }
        let letter = terms
            .first()
            .and_then(|t| t.term.chars().next())
            .unwrap_or('A');
        let group = GlossaryGroup {
            letter: letter.to_string(),
            anchor: letter.to_lowercase().to_string(),
            terms,
        };
        let page = RenderGlossary {
            title: lang.glossary.to_owned(),
            intro: None,
            groups: vec![group],
        };
        self.page(
            lang,
            (&glossary, None),
            lang.glossary,
            &PageBody::Glossary(page),
        )?;
        let path = format!("{}search/", lang.prefix);
        let search = RenderSearch {
            title: lang.search.to_owned(),
            action: path.clone(),
            query: None,
            results: Vec::new(),
        };
        self.page(lang, (&path, None), lang.search, &PageBody::Search(search))?;
        self.page(
            lang,
            (&format!("{}404.html", lang.prefix), None),
            "404",
            &PageBody::NotFound,
        )
    }

    /// Notes every post's path under its `translation` key, so each post
    /// can link to its versions in other languages.
    fn pair_translations(&mut self) -> Fallible<()> {
        for lang in &LANGUAGES {
            for (folder, prefix) in [("posts", ""), ("glossary", "glossary/")] {
                for doc in self.content(lang, folder)? {
                    let path = format!("{}{prefix}{}/", lang.prefix, doc.field("slug"));
                    let versions = self.translations.entry(doc.field("translation"));
                    versions.or_default().insert(lang.tag, path);
                }
            }
        }
        Ok(())
    }

    fn post(&mut self, lang: &Lang, post: &Doc, authors: &[Doc]) -> Fallible<PostCard> {
        let path = format!("{}{}/", lang.prefix, post.field("slug"));
        let author = post.field("author");
        let file = authors.iter().find(|a| a.field("slug") == author);
        let date = NaiveDate::parse_from_str(&post.field("date"), "%Y-%m-%d")?;
        let record = RenderPost {
            title: post.field("title"),
            date,
            updated: None,
            authors: vec![Person {
                name: file.map_or(author.clone(), |a| a.field("name")),
                href: format!("{}authors/{author}/", lang.prefix),
                role: file.map(|a| a.field("role")),
                image: None,
                bio: None,
                profiles: Vec::new(),
            }],
            reviewers: Vec::new(),
            category: Some(Link {
                name: self.category_name(lang, &post.field("category")),
                href: format!("{}category/{}/", lang.prefix, post.field("category")),
            }),
            tags: Vec::new(),
            image: None,
            body: markdown(&post.body, &format!("{}glossary/", lang.prefix)),
        };
        let (title, key) = (record.title.clone(), post.field("translation"));
        let body = PageBody::Post(Box::new(record));
        self.page(lang, (&path, Some(&key)), &title, &body)?;
        Ok(PostCard {
            title: post.field("title"),
            href: path,
            date,
            excerpt: post.field("description"),
            image: None,
        })
    }
}

#[expect(
    clippy::print_stdout,
    reason = "an example says where it wrote the pages"
)]
fn main() -> Fallible<()> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "target/demo-site".to_owned()),
    );
    let mut fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture.push("../../fixtures/demo-site");
    let css = stylesheet(&Tokens::load(&fixture)?)?;
    let mut file = out.clone();
    file.push(format!("blog/assets/{}", css.name()));
    fs::create_dir_all(file.parent().ok_or("no folder")?)?;
    fs::write(file, css.body())?;
    let mut demo = Demo {
        out: out.clone(),
        renderer: Renderer::new()?,
        stylesheet: format!("/blog/assets/{}", css.name()),
        pages: 0,
        translations: BTreeMap::new(),
        categories: category_names(&fixture)?,
        fixture,
    };
    demo.pair_translations()?;
    for lang in &LANGUAGES {
        demo.language(lang)?;
    }
    println!(
        "Wrote {} pages and the stylesheet to {}.",
        demo.pages,
        out.display()
    );
    println!(
        "Serve them with: python3 -m http.server 8000 --directory {}",
        out.display()
    );
    println!("Then open http://localhost:8000/blog/ in a browser.");
    Ok(())
}
