//! The decision records in `docs/adr/`: one file per decision. A record that
//! was renumbered must not come back under its old number, which a merge that
//! keeps one side of a stack whole has done twice (see
//! `docs/audits/2026-10-27-squash-reverts.md`).

use std::collections::BTreeMap;
use std::path::PathBuf;

/// The `docs/adr/` folder of this repository.
#[cfg(test)]
fn adr_folder() -> PathBuf {
    let mut folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    folder.push("../../docs/adr");
    folder
}

#[test]
fn no_decision_is_recorded_under_two_numbers() {
    let mut numbers_by_slug: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in std::fs::read_dir(adr_folder()).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        let Some((number, slug)) = name.strip_suffix(".md").and_then(|n| n.split_once('-')) else {
            continue;
        };
        numbers_by_slug
            .entry(slug.to_owned())
            .or_default()
            .push(number.to_owned());
    }

    let repeated: Vec<_> = numbers_by_slug
        .iter()
        .filter(|(_, numbers)| numbers.len() > 1)
        .collect();
    assert!(
        repeated.is_empty(),
        "these records exist under more than one number; keep the renumbered one: {repeated:?}"
    );
}
