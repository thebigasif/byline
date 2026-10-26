//! The empty config builds the same way through `new` and `Default`.

use byline_core::Config;

#[test]
fn new_matches_default() {
    assert_eq!(Config::new(), Config::default());
}
