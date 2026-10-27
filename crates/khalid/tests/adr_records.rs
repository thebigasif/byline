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

/// Every record in `docs/adr/` as (number, slug), sorted by file name.
#[cfg(test)]
fn records() -> Vec<(String, String)> {
    let mut records: Vec<(String, String)> = std::fs::read_dir(adr_folder())
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().into_string().unwrap();
            let (number, slug) = name.strip_suffix(".md")?.split_once('-')?;
            Some((number.to_owned(), slug.to_owned()))
        })
        .collect();
    records.sort();
    records
}

#[test]
fn no_decision_is_recorded_under_two_numbers() {
    let mut numbers_by_slug: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (number, slug) in records() {
        numbers_by_slug.entry(slug).or_default().push(number);
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

#[test]
fn no_number_holds_two_decisions() {
    let mut slugs_by_number: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (number, slug) in records() {
        // 0000 marks a record whose pull request has not landed yet.
        if number != "0000" {
            slugs_by_number.entry(number).or_default().push(slug);
        }
    }

    let shared: Vec<_> = slugs_by_number
        .iter()
        .filter(|(_, slugs)| slugs.len() > 1)
        .collect();
    assert!(
        shared.is_empty(),
        "these numbers hold more than one record; the later one takes the next free number: {shared:?}"
    );
}

#[test]
fn the_index_lists_every_numbered_record_once_in_order() {
    let mut numbered: Vec<String> = records()
        .into_iter()
        .map(|(number, _)| number)
        .filter(|number| number != "0000")
        .collect();
    numbered.dedup();

    let mut readme = adr_folder();
    readme.push("README.md");
    let index: Vec<String> = std::fs::read_to_string(readme)
        .unwrap()
        .lines()
        .filter_map(|line| line.strip_prefix("| ")?.split_once(' '))
        .map(|(number, _)| number.to_owned())
        .filter(|number| number.len() == 4 && number.bytes().all(|b| b.is_ascii_digit()))
        .collect();

    assert_eq!(
        index, numbered,
        "docs/adr/README.md must list each record once, sorted by number"
    );
}
