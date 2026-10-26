//! `meta.pagination` on every list, in Ghost's shape:
//! `{"page": 1, "limit": 15, "pages": 4, "total": 52, "next": 2, "prev": null}`.

use khalid_api::pagination::Meta;
use khalid_api::query::Limit;
use serde_json::json;

#[test]
fn a_middle_page_has_both_neighbours() {
    let meta = Meta::new(2, Limit::Count(15), 52);
    assert_eq!(
        serde_json::to_value(&meta).unwrap(),
        json!({"pagination": {"page": 2, "limit": 15, "pages": 4, "total": 52, "next": 3, "prev": 1}})
    );
    assert_eq!(meta.pagination().offset(), 15);
}

#[test]
fn the_first_and_last_pages_lack_one_neighbour() {
    let first = Meta::new(1, Limit::Count(15), 52);
    let last = Meta::new(4, Limit::Count(15), 52);
    let json = |meta: &Meta| serde_json::to_value(meta).unwrap()["pagination"].clone();
    assert_eq!(
        (json(&first)["next"].clone(), json(&first)["prev"].clone()),
        (json!(2), json!(null))
    );
    assert_eq!(
        (json(&last)["next"].clone(), json(&last)["prev"].clone()),
        (json!(null), json!(3))
    );
}

#[test]
fn an_empty_list_has_one_empty_page() {
    let meta = Meta::new(1, Limit::Count(15), 0);
    assert_eq!(
        serde_json::to_value(&meta).unwrap()["pagination"],
        json!({"page": 1, "limit": 15, "pages": 1, "total": 0, "next": null, "prev": null})
    );
}

#[test]
fn limit_all_is_one_page_and_says_all() {
    let meta = Meta::new(1, Limit::All, 52);
    assert_eq!(
        serde_json::to_value(&meta).unwrap()["pagination"],
        json!({"page": 1, "limit": "all", "pages": 1, "total": 52, "next": null, "prev": null})
    );
    assert_eq!(meta.pagination().offset(), 0);
}

#[test]
fn a_page_past_the_end_points_back() {
    let meta = Meta::new(9, Limit::Count(15), 52);
    let pagination = &serde_json::to_value(&meta).unwrap()["pagination"];
    assert_eq!(
        (pagination["next"].clone(), pagination["prev"].clone()),
        (json!(null), json!(8))
    );
    assert_eq!(meta.pagination().offset(), 120);
}

#[test]
fn the_largest_page_does_not_overflow() {
    let meta = Meta::new(1_000_000, Limit::Count(100), u64::MAX);
    assert_eq!(meta.pagination().offset(), 99_999_900);
}
