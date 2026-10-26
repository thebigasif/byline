//! The fields of a form block.

/// One labelled field of a form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormField {
    /// What the field takes.
    pub kind: FieldKind,
    /// The name the server receives the value under.
    pub name: String,
    /// The words beside the field.
    pub label: String,
    /// Whether the form cannot be sent without it.
    pub required: bool,
}

/// What a form field takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldKind {
    /// One line of text.
    Text,
    /// An email address.
    Email,
    /// Several lines of text.
    Textarea,
    /// A phone number.
    Tel,
    /// A web address.
    Url,
    /// A number.
    Number,
}

impl FieldKind {
    /// The kind written `name` in a form block, such as `email`.
    pub(crate) fn named(name: &str) -> Option<Self> {
        match name {
            "text" => Some(Self::Text),
            "email" => Some(Self::Email),
            "textarea" => Some(Self::Textarea),
            "tel" => Some(Self::Tel),
            "url" => Some(Self::Url),
            "number" => Some(Self::Number),
            _ => None,
        }
    }

    /// The HTML input type, or `None` for a `<textarea>`.
    pub(crate) const fn input_type(self) -> Option<&'static str> {
        match self {
            Self::Text => Some("text"),
            Self::Email => Some("email"),
            Self::Textarea => None,
            Self::Tel => Some("tel"),
            Self::Url => Some("url"),
            Self::Number => Some("number"),
        }
    }
}
