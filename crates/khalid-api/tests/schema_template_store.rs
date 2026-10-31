//! Schema templates as the Admin API holds them (G54, KH-SEO-4): each is
//! checked as `khalid-seo` writes it, and a change on an old copy is
//! refused.

use chrono::{TimeDelta, TimeZone, Utc};
use khalid_api::resources::NonBlank;
use khalid_api::schema_template_store::{
    AdminSchemaTemplate, MemorySchemaTemplates, SchemaTemplateStore, SchemaTemplates,
    TemplateStoreError, TemplateTarget, TemplatesChange,
};

/// A `Recipe` for every post in `posts`, with `properties`.
#[cfg(test)]
fn recipe(kind: &str, properties: &[(&str, &str)]) -> AdminSchemaTemplate {
    let posts = NonBlank::try_new("posts").unwrap();
    let mut template = AdminSchemaTemplate::new(TemplateTarget::Collection, posts, kind);
    for (name, value) in properties {
        template
            .properties
            .insert((*name).to_owned(), (*value).to_owned());
    }
    template
}

#[test]
fn a_template_checks_its_type_property_names_and_variables() {
    let good = recipe("Recipe", &[("name", "{title}"), ("author", "{author}")]);
    let checked = good.checked().unwrap();
    assert_eq!(checked.kind().as_str(), "Recipe");
    let refused = [
        (recipe("recipe", &[]), "schema.org type"),
        (recipe("Recipe", &[("Name", "{title}")]), "property name"),
        (recipe("Recipe", &[("name", "{headline}")]), "headline"),
        (recipe("Recipe", &[("name", "{title")]), "brace"),
    ];
    for (template, words) in refused {
        let why = template.checked().unwrap_err();
        assert!(why.contains(words), "{template:?}: {why}");
    }
}

#[tokio::test]
async fn a_change_on_an_old_copy_changes_nothing() {
    let at = Utc.with_ymd_and_hms(2026, 10, 30, 9, 0, 0).unwrap();
    let store = MemorySchemaTemplates::new(SchemaTemplates {
        templates: Vec::new(),
        collections: vec!["posts".to_owned()],
        updated_at: at,
        restart_needed: false,
    });
    let old = TemplatesChange {
        templates: vec![recipe("Recipe", &[])],
        updated_at: at - TimeDelta::seconds(5),
    };
    let refused = store.replace(old).await.unwrap_err();
    assert!(matches!(refused, TemplateStoreError::Stale { updated_at } if updated_at == at));
    assert_eq!(store.read().await.unwrap().templates, Vec::new());
}
