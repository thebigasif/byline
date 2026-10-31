//! The plain pages the form route answers with, in English.

use std::fmt::Write as _;

use crate::submission::{LONGEST_ANSWER, Problem};

/// What the reader is told after posting a form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answer {
    /// The answers were kept, or a robot was sent away quietly.
    Sent,
    /// The site has no such form.
    NotFound,
    /// The answers could not be kept as sent.
    Problem(Problem),
    /// One address sent too many; it may send again in `minutes`.
    TooMany { minutes: u64 },
    /// The answers could not be stored.
    Unavailable,
}

/// The page for `answer`, linking `back` and the site's `stylesheet`.
pub(crate) fn page(answer: Answer, back: &str, stylesheet: &str) -> String {
    let (title, lead, more) = words(answer);
    let mut html = String::with_capacity(640);
    let _ = write!(
        html,
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"robots\" content=\"noindex\">\n<title>{title}</title>\n\
         <link rel=\"stylesheet\" href=\"{}\">\n</head>\n<body>\n<main>\n\
         <h1>{lead}</h1>\n<p>{more}</p>\n<p><a href=\"{}\">Back to the page</a></p>\n\
         </main>\n</body>\n</html>\n",
        escape(stylesheet),
        escape(back),
    );
    html
}

/// The page's title, its heading and the sentence under it.
fn words(answer: Answer) -> (&'static str, String, String) {
    let not_sent = || "Your message was not sent.".to_owned();
    match answer {
        Answer::Sent => (
            "Message sent",
            "Thank you. Your message was sent.".to_owned(),
            "The site's owner will see it.".to_owned(),
        ),
        Answer::NotFound => (
            "Form not found",
            "This form is not on the site.".to_owned(),
            "It may have been taken off its page. Go back to the site to find what you were looking for.".to_owned(),
        ),
        Answer::Problem(Problem::Empty) => (
            "Message not sent",
            not_sent(),
            "Every field was empty. Go back, fill in the form and send it again.".to_owned(),
        ),
        Answer::Problem(Problem::TooLong) => (
            "Message not sent",
            not_sent(),
            format!(
                "One answer is longer than {LONGEST_ANSWER} characters. Go back, shorten it and send it again."
            ),
        ),
        Answer::Problem(Problem::Unreadable) => (
            "Message not sent",
            not_sent(),
            "The form could not be read. Go back to the page and send it again.".to_owned(),
        ),
        Answer::TooMany { minutes } => {
            let wait = if minutes == 1 {
                "1 minute".to_owned()
            } else {
                format!("{minutes} minutes")
            };
            (
                "Please wait",
                format!("Please wait {wait}."),
                format!(
                    "Several messages came from your network in a short time, so this one was not sent. Send it again after {wait}."
                ),
            )
        }
        Answer::Unavailable => (
            "Message not sent",
            not_sent(),
            "The site could not save it just now. Please try again in a few minutes.".to_owned(),
        ),
    }
}

/// `text` safe inside a double-quoted attribute.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_escaped() {
        let html = page(Answer::Sent, "/a\"><script>", "/s.css");
        assert!(
            html.contains("href=\"/a&quot;&gt;&lt;script&gt;\""),
            "{html}"
        );
    }

    #[test]
    fn one_minute_is_said_as_one() {
        let one = page(Answer::TooMany { minutes: 1 }, "/", "/s.css");
        assert!(one.contains("Please wait 1 minute."), "{one}");
        let ten = page(Answer::TooMany { minutes: 10 }, "/", "/s.css");
        assert!(ten.contains("after 10 minutes."), "{ten}");
    }
}
