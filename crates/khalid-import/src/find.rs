//! Finding elements and their text in a parsed HTML body, for the
//! importers that read HTML: Medium and Substack.

use scraper::{ElementRef, Selector};

/// Every element `css` selects inside `element`, in order.
pub(crate) fn select<'a>(element: ElementRef<'a>, css: &str) -> Vec<ElementRef<'a>> {
    Selector::parse(css)
        .map(|selector| element.select(&selector).collect())
        .unwrap_or_default()
}

/// The first element `css` selects inside `element`.
pub(crate) fn first<'a>(element: ElementRef<'a>, css: &str) -> Option<ElementRef<'a>> {
    select(element, css).into_iter().next()
}

/// The element's text, with every run of spaces and line breaks made one
/// space.
pub(crate) fn words(element: ElementRef<'_>) -> String {
    let text: String = element.text().collect();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether the element has the class `name`.
pub(crate) fn has_class(element: ElementRef<'_>, name: &str) -> bool {
    element.value().classes().any(|class| class == name)
}
