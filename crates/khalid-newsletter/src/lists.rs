//! The newsletters one site has (task 5.7).

use std::collections::BTreeSet;

use khalid_core::Slug;

/// The name of the newsletter every site has, which a form without
/// `list=` joins.
const MAIN: &str = "main";

/// The newsletters of one site: `main`, and any others the site names.
/// A form can only join one of these, so a reader cannot make up a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lists {
    names: BTreeSet<Slug>,
}

impl Lists {
    /// The site's `main` newsletter alone.
    pub fn main_only() -> Self {
        Self {
            names: Slug::try_new(MAIN).into_iter().collect(),
        }
    }

    /// `main` and every newsletter in `names`.
    pub fn new(names: impl IntoIterator<Item = Slug>) -> Self {
        let mut lists = Self::main_only();
        lists.names.extend(names);
        lists
    }

    /// The newsletter called `name` as a form sends it, or `main` when the
    /// form names none. `None` when the site has no newsletter by that name.
    pub fn find(&self, name: Option<&str>) -> Option<Slug> {
        let name = name.filter(|name| !name.is_empty()).unwrap_or(MAIN);
        self.names
            .iter()
            .find(|list| list.as_str() == name)
            .cloned()
    }

    /// The newsletters, in alphabetical order.
    pub fn iter(&self) -> impl Iterator<Item = &Slug> {
        self.names.iter()
    }
}

impl Default for Lists {
    fn default() -> Self {
        Self::main_only()
    }
}
