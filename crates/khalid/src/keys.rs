//! `khalid keys create` and `khalid keys revoke`: API keys in the site's
//! `data/site.db`, stored as hashes only (RULES.md BL-SEC-3, ADR 0011).

use std::path::Path;

use khalid_api::keys::{KeyHashing, KeyId, KeyKind, KeyStore, KeyToken, Role, Scope, StoredKey};
use khalid_core::{Slug, SystemClock, UserMessage};
use khalid_db::{Database, KeyTable};
use serde::Serialize;

use crate::cli::{KeyKindArg, KeysCommand, NewKeyArgs, Output};
use crate::exit::Exit;
use crate::{db, db_message, keys_message, report};

/// What a new key may do, once the flags are checked.
enum Grant {
    Content,
    Integration(Vec<Scope>),
    Staff(Role),
}

/// The JSON shape of `khalid keys create --json`. `key` appears here and
/// nowhere else, ever.
#[derive(Serialize)]
struct CreatedBody<'a> {
    id: &'a str,
    kind: &'a str,
    name: &'a str,
    key: &'a str,
}

/// The JSON shape of `khalid keys revoke --json`.
#[derive(Serialize)]
struct RevokedBody<'a> {
    id: &'a str,
    revoked: bool,
}

/// Runs a key command on the `site` folder, whose settings were checked.
pub(crate) fn run(site: &Path, command: &KeysCommand, output: Output) -> Exit {
    match command {
        KeysCommand::Create(args) => create(site, args, output),
        KeysCommand::Revoke { id } => revoke(site, id, output),
    }
}

/// Makes a key, stores its hash and shows the key once.
fn create(site: &Path, args: &NewKeyArgs, output: Output) -> Exit {
    let grant = match checked(args) {
        Ok(grant) => grant,
        Err(message) => return report::failure(output, &message, Exit::Usage),
    };
    let kind = match grant {
        Grant::Content => KeyKind::Content,
        Grant::Integration(_) => KeyKind::Integration,
        Grant::Staff(_) => KeyKind::Staff,
    };
    let Ok(token) = KeyToken::generate(kind) else {
        return report::failure(output, &keys_message::no_randomness(), Exit::Internal);
    };
    let hashing = KeyHashing::new();
    let stored = match grant {
        Grant::Content => Ok(StoredKey::for_token(&token)),
        Grant::Integration(scopes) => StoredKey::for_integration(&token, scopes, &hashing),
        Grant::Staff(role) => StoredKey::for_staff(&token, role, &hashing),
    };
    let Ok(stored) = stored else {
        return report::failure(output, &keys_message::internal(), Exit::Internal);
    };
    let saved = on_table(site, async |table| table.create(stored, &args.name).await);
    match saved {
        Ok(Ok(())) => shown(output, token, args),
        Ok(Err(_)) => report::failure(output, &keys_message::not_saved(site), Exit::Failure),
        Err(message) => report::failure(output, &message, Exit::Failure),
    }
}

/// Checks that the flags give the kind of key exactly what it needs:
/// scopes for an integration key, a role for a staff token, neither for a
/// content key.
fn checked(args: &NewKeyArgs) -> Result<Grant, UserMessage> {
    if args.name.trim().is_empty() {
        return Err(keys_message::no_name());
    }
    match (args.kind, args.scopes.as_slice(), args.role) {
        (KeyKindArg::Content, [], None) => Ok(Grant::Content),
        (KeyKindArg::Integration, [_, ..], None) => Ok(Grant::Integration(args.scopes.clone())),
        (KeyKindArg::Staff, [], Some(role)) => Ok(Grant::Staff(role)),
        (KeyKindArg::Content, ..) => Err(keys_message::content_takes_nothing()),
        (KeyKindArg::Integration, ..) => Err(keys_message::integration_needs_scopes()),
        (KeyKindArg::Staff, ..) => Err(keys_message::staff_needs_a_role()),
    }
}

/// Prints the new key: the one time it is ever shown.
fn shown(output: Output, token: KeyToken, args: &NewKeyArgs) -> Exit {
    let id = token.id().as_str().to_owned();
    let kind = match args.kind {
        KeyKindArg::Content => "content",
        KeyKindArg::Integration => "integration",
        KeyKindArg::Staff => "staff",
    };
    let key = token.reveal();
    match output {
        Output::Json => report::json(
            &CreatedBody {
                id: &id,
                kind,
                name: &args.name,
                key: key.expose(),
            },
            Exit::Success,
        ),
        Output::Human => report::text(
            &format!(
                "Made the {kind} key for {name}, with the identity {id}.\n\
                 This is the only time Khalid shows it, so copy it now into what uses it:\n\n    \
                 {key}\n\n\
                 To stop it working, run `khalid keys revoke {id}`.\n",
                name = args.name,
                key = key.expose(),
            ),
            Exit::Success,
        ),
    }
}

/// Revokes the key with identity `id`.
fn revoke(site: &Path, id: &KeyId, output: Output) -> Exit {
    match on_table(site, async |table| table.revoke(id).await) {
        Ok(Ok(true)) => match output {
            Output::Json => report::json(
                &RevokedBody {
                    id: id.as_str(),
                    revoked: true,
                },
                Exit::Success,
            ),
            Output::Human => report::text(
                &format!(
                    "Revoked the key {}. It stopped working at once.\n",
                    id.as_str()
                ),
                Exit::Success,
            ),
        },
        Ok(Ok(false)) => report::failure(output, &keys_message::unknown(id), Exit::Failure),
        Ok(Err(_)) => report::failure(output, &keys_message::not_saved(site), Exit::Failure),
        Err(message) => report::failure(output, &message, Exit::Failure),
    }
}

/// Opens the site's database, brings it up to date, runs `work` on the key
/// table of the site, then closes the database.
fn on_table<T>(site: &Path, work: impl AsyncFnOnce(&KeyTable) -> T) -> Result<T, UserMessage> {
    let path = db::data_file(site)?;
    let (Ok(main), Ok(runtime)) = (
        Slug::try_new(db::MAIN),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return Err(db_message::internal());
    };
    runtime.block_on(async {
        let database = Database::open(&path)
            .await
            .map_err(|error| db_message::of(&path, &error))?;
        let opened = async {
            database.migrate().await?;
            database.site(&main).await
        }
        .await;
        let done = match opened {
            Ok(id) => Ok(work(&KeyTable::new(database.clone(), id, SystemClock)).await),
            Err(error) => Err(db_message::of(&path, &error)),
        };
        database.close().await;
        done
    })
}
