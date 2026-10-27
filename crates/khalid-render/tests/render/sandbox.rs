//! The sandbox (RULES.md BL-SEC-10) and theme template replacements.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_core::TemplateName;
use khalid_render::{PageBody, PageKind, RenderError, Renderer};

use crate::common::{demo_site, meta};

/// A site folder whose `theme/templates/` holds `files`.
fn site_with_theme(files: &[(&str, &str)]) -> tempfile::TempDir {
    let site = tempfile::tempdir().unwrap();
    let folder = in_folder(site.path(), "theme/templates");
    fs::create_dir_all(&folder).unwrap();
    for (name, text) in files {
        fs::write(in_folder(&folder, name), text).unwrap();
    }
    site
}

fn in_folder(folder: &Path, relative: &str) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push(relative);
    path
}

fn render_with(site_root: &Path) -> Result<String, RenderError> {
    let meta = meta("Page not found", "/blog/missing/", PageKind::Website);
    Renderer::with_theme(site_root)?.render(&demo_site("en"), &meta, &PageBody::NotFound)
}

#[test]
fn a_site_without_a_theme_folder_uses_the_defaults() {
    let site = tempfile::tempdir().unwrap();
    let page = render_with(site.path()).unwrap();
    assert!(page.contains("This page does not exist"));
}

#[test]
fn a_theme_template_replaces_the_default_of_the_same_name() {
    let site = site_with_theme(&[(
        "not_found.html",
        r#"{% extends "base.html" %}{% block main %}<h1>Lost? {{ site.name }}</h1>{% endblock %}"#,
    )]);
    let page = render_with(site.path()).unwrap();
    assert!(page.contains("<h1>Lost? Field Notes on SEO</h1>"));
}

#[test]
fn a_theme_file_that_replaces_nothing_is_refused() {
    for name in [
        "missing_page.html",
        "post-Wide.html",
        "post-.html",
        "post-wide.txt",
    ] {
        let site = site_with_theme(&[(name, "a name no default has")]);
        let error = render_with(site.path()).unwrap_err();
        assert!(
            matches!(&error, RenderError::UnknownTemplate { name: got } if got == name),
            "{error:?}"
        );
    }
}

#[test]
fn a_post_template_draws_the_page_a_post_asks_for() {
    let wide = r#"{% extends "base.html" %}{% block main %}<h1 class="wide">{{ site.name }}</h1>{% endblock %}"#;
    let site = site_with_theme(&[("post-wide.html", wide)]);
    let renderer = Renderer::with_theme(site.path()).unwrap();
    let meta = meta("Notes", "/blog/notes/", PageKind::Article);
    let name = TemplateName::try_new("wide").unwrap();
    assert!(renderer.has_template(&name));
    let page = renderer.render_as(&name, &demo_site("en"), &meta, &PageBody::NotFound);
    assert!(
        page.unwrap()
            .contains("<h1 class=\"wide\">Field Notes on SEO</h1>")
    );
    let narrow = TemplateName::try_new("narrow").unwrap();
    assert!(!renderer.has_template(&narrow));
    let missing = renderer.render_as(&narrow, &demo_site("en"), &meta, &PageBody::NotFound);
    assert!(
        matches!(&missing, Err(RenderError::Render { name, .. }) if name == "post-narrow.html"),
        "{missing:?}"
    );
}

#[test]
fn a_syntax_mistake_names_the_template_and_line() {
    let site = site_with_theme(&[("not_found.html", "<p>fine</p>\n{% if %}")]);
    let error = render_with(site.path()).unwrap_err();
    assert!(
        matches!(&error, RenderError::Compile { name, line: Some(2), .. } if name == "not_found.html"),
        "{error:?}"
    );
}

#[test]
fn a_value_the_page_does_not_have_is_an_error_not_an_empty_string() {
    let site = site_with_theme(&[("not_found.html", "{{ site.missing_field }}")]);
    let error = render_with(site.path()).unwrap_err();
    assert!(matches!(&error, RenderError::Render { name, .. } if name == "not_found.html"));
}

#[test]
fn templates_cannot_read_files() {
    let site = site_with_theme(&[("not_found.html", r#"{% include "/etc/passwd" %}"#)]);
    assert!(matches!(
        render_with(site.path()),
        Err(RenderError::Render { .. })
    ));
}

#[test]
fn templates_cannot_call_functions_the_crate_did_not_give_them() {
    let site = site_with_theme(&[("not_found.html", r#"{{ open("site.toml") }}"#)]);
    assert!(matches!(
        render_with(site.path()),
        Err(RenderError::Render { .. })
    ));
}

#[test]
fn a_template_that_loops_without_end_is_stopped() {
    let endless = "{% for a in range(10000) %}{% for b in range(10000) %}{% endfor %}{% endfor %}";
    let site = site_with_theme(&[("not_found.html", endless)]);
    assert!(matches!(
        render_with(site.path()),
        Err(RenderError::Render { .. })
    ));
}

#[cfg(unix)]
#[test]
fn a_theme_folder_linked_outside_the_site_is_refused() {
    let outside = tempfile::tempdir().unwrap();
    let site = tempfile::tempdir().unwrap();
    fs::create_dir(in_folder(site.path(), "theme")).unwrap();
    std::os::unix::fs::symlink(outside.path(), in_folder(site.path(), "theme/templates")).unwrap();
    assert!(matches!(
        render_with(site.path()),
        Err(RenderError::ThemePath(_))
    ));
}
