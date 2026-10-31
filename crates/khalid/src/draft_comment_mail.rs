//! Comments on drafts as `khalid serve` keeps them, and the email each new
//! one sends to the site's editors and the people in its thread (KH-EDT-4,
//! plan section 4.6).

use std::sync::Arc;

use khalid_api::content_store::BoxFuture;
use khalid_api::draft_comments::{DraftComment, DraftCommentNotice, DraftComments};
use khalid_api::keys::Role;
use khalid_api::resources::AdminPost;
use khalid_auth::{Account, AccountStore};
use khalid_core::{Config, SystemClock};
use khalid_db::{AccountTable, Database, DraftCommentTable, SiteId};
use khalid_newsletter::{EmailAddress, Mail, Mailer};

use crate::staff;

/// The comments on drafts of site `id`, emailed through `mailer` when the
/// site sends mail.
pub(crate) fn desk(
    (database, id, config): (&Database, SiteId, &Config),
    mailer: Option<Arc<dyn Mailer>>,
) -> DraftComments {
    let store = DraftCommentTable::new(database.clone(), id, SystemClock);
    let comments = DraftComments::new(Arc::new(store));
    match mailer {
        Some(mailer) => comments.with_notice(Arc::new(CommentMail {
            accounts: AccountTable::new(database.clone(), id),
            mailer,
            admin: staff::admin_address(config),
        })),
        None => comments,
    }
}

/// Emails each new comment, in the background so the writer never waits
/// on the mail server.
struct CommentMail {
    accounts: AccountTable,
    mailer: Arc<dyn Mailer>,
    admin: String,
}

impl DraftCommentNotice for CommentMail {
    fn added<'a>(
        &'a self,
        post: &'a AdminPost,
        comment: &'a DraftComment,
        earlier: &'a [DraftComment],
    ) -> BoxFuture<'a, ()> {
        let accounts = self.accounts.clone();
        let mailer = Arc::clone(&self.mailer);
        let thread = in_thread(comment, earlier);
        let (title, admin) = (post.title().to_owned(), self.admin.clone());
        let comment = comment.clone();
        tokio::spawn(async move {
            let all = match accounts.list().await {
                Ok(all) => all,
                Err(error) => {
                    tracing::warn!(
                        %error,
                        "the staff accounts could not be read to email a comment"
                    );
                    return;
                }
            };
            for mail in mails(&all, &comment, &thread, (&title, &admin)) {
                // Only the error's own words: a transport's cause may name
                // the address, which never goes to the log (BL-RUN-5).
                if let Err(error) = mailer.send(&mail).await {
                    tracing::warn!(%error, "a comment on a draft could not be emailed");
                }
            }
        });
        Box::pin(async {})
    }
}

/// Who wrote the comments before `comment` in its thread.
fn in_thread(comment: &DraftComment, earlier: &[DraftComment]) -> Vec<String> {
    let Some(first) = comment.parent else {
        return Vec::new();
    };
    let thread = earlier
        .iter()
        .filter(|c| c.id == first || c.parent == Some(first));
    thread.map(|c| c.author.clone()).collect()
}

/// How an account is named as a comment's author: its row as 16 hex
/// digits, the shape of a key's identity.
fn author_id(account: &Account) -> String {
    format!("{:016x}", account.id().as_row())
}

/// The emails about `comment` on the post titled `title`: one to each
/// editor, admin and owner, and to each account in `thread`, never to its
/// writer. `admin` is where the admin is served.
fn mails(
    accounts: &[Account],
    comment: &DraftComment,
    thread: &[String],
    (title, admin): (&str, &str),
) -> Vec<Mail> {
    let writer = accounts.iter().find(|a| author_id(a) == comment.author);
    let who = writer.map_or("Someone using an API key", Account::name);
    let about = comment
        .block
        .map_or(String::new(), |block| format!(", on block {block}"));
    let subject = format!("New comment on \"{title}\"");
    let text = format!(
        "{who} commented on \"{title}\"{about}:\n\n{}\n\nAnswer it in the admin: {admin}\n",
        comment.text.as_str()
    );
    let editor = |a: &Account| matches!(a.role(), Role::Owner | Role::Admin | Role::Editor);
    accounts
        .iter()
        .filter(|a| author_id(a) != comment.author)
        .filter(|a| editor(a) || thread.contains(&author_id(a)))
        .filter_map(|a| EmailAddress::parse(a.email().as_str()).ok())
        .map(|to| Mail::new(to, subject.clone(), text.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use khalid_api::draft_comments::{DraftCommentText, NewDraftComment};
    use khalid_auth::AccountId;

    use super::*;

    fn account(row: i64, email: &str, role: Role) -> Account {
        let name = email.split('@').next().unwrap_or_default().to_owned();
        Account::new(AccountId::from_row(row), email.parse().unwrap(), name, role)
    }

    fn comment(id: i64, author: i64, parent: Option<i64>) -> DraftComment {
        let text = DraftCommentText::try_from("Name the tool here?".to_owned()).unwrap();
        let mut new = NewDraftComment::new("en.notes", &format!("{author:016x}"), text);
        (new.parent, new.block) = (parent, Some(2));
        let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
        DraftComment::saved(id, new, at)
    }

    #[test]
    fn editors_and_the_thread_hear_of_a_comment_and_its_writer_does_not() {
        let staff = [
            account(1, "owner@example.com", Role::Owner),
            account(2, "mira@example.com", Role::Editor),
            account(3, "sam@example.com", Role::Writer),
            account(4, "lee@example.com", Role::Writer),
        ];
        let first = comment(1, 3, None);
        let reply = comment(2, 2, Some(1));
        let thread = in_thread(&reply, &[first, comment(9, 4, None)]);
        let sent = mails(
            &staff,
            &reply,
            &thread,
            ("Notes", "https://example.com/blog/admin"),
        );
        let to: Vec<String> = sent.iter().map(|m| m.to.to_string()).collect();
        assert_eq!(to, ["owner@example.com", "sam@example.com"]);
        assert_eq!(sent[0].subject, "New comment on \"Notes\"");
        assert_eq!(
            sent[0].text,
            "mira commented on \"Notes\", on block 2:\n\nName the tool here?\n\n\
             Answer it in the admin: https://example.com/blog/admin\n"
        );
    }
}
