//! The publish gate in `khalid build`: every post that would go live must
//! pass before any page is written, so a refused post never leaves half a
//! site behind.
//!
//! A post with no date is a draft. `khalid build` leaves it out, so the gate
//! leaves it alone too.

use khalid_content::SiteContent;
use khalid_core::{Config, UserMessage};
use khalid_lint::{PublishGate, Report, Verdict};
use khalid_routes::Routes;
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::lint_input::{self, LintInput};
use crate::site_briefs::Briefs;
use crate::{lint, report};

/// The JSON shape of a refused build: the message, then each refused post's
/// findings, warnings included.
#[derive(Serialize)]
struct RefusedBody<'a> {
    error: &'a UserMessage,
    refused: &'a [Report],
}

/// The report of every dated post in `content` that `gate` refuses, in the
/// order the posts were read. Empty when the site may be built. Each post
/// first gets the quick yes or no, and only a refused post the full verdict
/// with its warnings, so a 10,000-post build stays inside its budget. Posts
/// are held to their `briefs`.
pub(crate) fn refused(
    gate: &dyn PublishGate,
    content: &SiteContent,
    config: &Config,
    routes: &Routes,
    briefs: Briefs,
) -> Vec<Report> {
    let input = LintInput::new(content, config, routes.clone()).with_briefs(briefs);
    let (site, _) = lint_input::lint_site(content, config, &input);
    site.posts
        .iter()
        .filter(|post| post.date.is_some() && gate.refuses(post, &site))
        .filter_map(|post| match gate.verdict(post, &site) {
            Verdict::Refuse { report } => Some(report),
            Verdict::Publish { .. } => None,
        })
        .collect()
}

/// Explains why nothing was built: every finding of each refused post, then
/// what, why and fix. Ends with exit code 1.
pub(crate) fn refuse(output: Output, refused: &[Report]) -> Exit {
    let message = not_ready(refused.len());
    match output {
        Output::Json => report::json(
            &RefusedBody {
                error: &message,
                refused,
            },
            Exit::Failure,
        ),
        Output::Human => {
            let mut text = String::new();
            for finding in refused.iter().flat_map(Report::findings) {
                lint::write_finding(&mut text, finding);
            }
            match report::text(&text, Exit::Failure) {
                Exit::Failure => report::failure(output, &message, Exit::Failure),
                unwritten => unwritten,
            }
        }
    }
}

/// The message for a build the gate stopped, with `posts` refused.
fn not_ready(posts: usize) -> UserMessage {
    let count = if posts == 1 {
        "1 post is".to_owned()
    } else {
        format!("{posts} posts are")
    };
    UserMessage::new(
        format!("Khalid built nothing, because {count} not ready to publish."),
        "A blocked post would go live with its problems, so no page was written.",
        "Fix each block listed with its post, or remove `date:` to keep a post as an unpublished draft.",
    )
}
