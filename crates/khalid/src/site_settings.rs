//! The Admin API's core settings on a running site: `site.toml`, edited in
//! place so its comments and tables stay as written, checked as `khalid
//! serve` checks it at start before anything is written, and written
//! atomically (RULES.md BL-RUN-9). The running server keeps the settings
//! it started with, so a saved change applies after a restart.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::entry_store::EntryStoreError;
use khalid_api::resources::{AdminSettings, SettingsChange};
use khalid_api::settings_store::{SettingsError, SettingsStore};
use khalid_core::{Config, ConfigError, Environment};
use toml_edit::{Array, DocumentMut, Item, Value};

use crate::atomic;
use crate::site_config;

/// The settings of one site folder. Cloning shares it.
#[derive(Debug, Clone)]
pub(crate) struct SiteSettings {
    site: PathBuf,
    /// The core settings the server started with.
    running: Core,
    /// Held while a change is checked and written, so no two interleave.
    writing: Arc<Mutex<()>>,
}

/// The core settings, as a change may set them.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Core {
    name: String,
    address: String,
    mount: String,
    default_language: String,
    languages: Vec<String>,
}

impl Core {
    fn of(config: &Config) -> Self {
        Self {
            name: config.name().to_owned(),
            address: config.address().as_str().to_owned(),
            mount: config.mount().to_owned(),
            default_language: config.default_language().as_str().to_owned(),
            languages: config
                .languages()
                .iter()
                .map(|l| l.as_str().to_owned())
                .collect(),
        }
    }
}

impl SiteSettings {
    /// The settings in `site`'s `site.toml`, for a server that started with
    /// `running`. Every step runs on a blocking thread (RULES.md BL-RUN-7).
    pub(crate) fn new(site: &Path, running: &Config) -> Self {
        Self {
            site: site.to_path_buf(),
            running: Core::of(running),
            writing: Arc::new(Mutex::new(())),
        }
    }

    fn path(&self) -> PathBuf {
        let mut path = self.site.clone();
        path.push(Config::FILE_NAME);
        path
    }

    /// The file's text and when it last changed.
    fn text(&self) -> Result<(String, DateTime<Utc>), EntryStoreError> {
        let path = self.path();
        let text = fs::read_to_string(&path).map_err(EntryStoreError::new)?;
        let changed = fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .map_err(EntryStoreError::new)?;
        Ok((text, changed.into()))
    }

    /// The settings `text` holds, checked, as last changed at `changed`.
    fn settings(&self, text: &str, changed: DateTime<Utc>) -> Result<AdminSettings, SettingsError> {
        let config = self.checked(text)?;
        let core = Core::of(&config);
        let restart_needed = core != self.running;
        Ok(AdminSettings {
            name: core.name,
            address: core.address,
            mount: core.mount,
            default_language: core.default_language,
            languages: core.languages,
            updated_at: changed,
            restart_needed,
        })
    }

    /// `text` as a config, when it passes every check made at start.
    fn checked(&self, text: &str) -> Result<Config, SettingsError> {
        let config = Config::from_toml(&self.path(), text, &Environment::empty());
        let config = config.map_err(refused)?;
        site_config::check(&self.site, &config).map_err(|message| SettingsError::Refused {
            field: "address".to_owned(),
            problem: message.what().to_owned(),
        })?;
        Ok(config)
    }

    fn read_now(&self) -> Result<AdminSettings, EntryStoreError> {
        let (text, changed) = self.text()?;
        self.settings(&text, changed).map_err(EntryStoreError::new)
    }

    fn change_now(&self, change: SettingsChange) -> Result<AdminSettings, SettingsError> {
        let _writing = self.lock();
        let (text, changed) = self.text()?;
        if changed != change.updated_at {
            return Err(SettingsError::Stale {
                updated_at: changed,
            });
        }
        let mut document = text.parse::<DocumentMut>().map_err(EntryStoreError::new)?;
        let texts = [
            ("name", change.name),
            ("address", change.address),
            ("mount", change.mount),
            ("default_language", change.default_language),
        ];
        for (key, sent) in texts {
            if let Some(sent) = sent {
                set(&mut document, key, sent.into());
            }
        }
        if let Some(languages) = change.languages {
            let languages: Array = languages.into_iter().collect();
            set(&mut document, "languages", languages.into());
        }
        let text = document.to_string();
        self.checked(&text)?;
        atomic::replace(&self.path(), &text).map_err(EntryStoreError::new)?;
        let (text, changed) = self.text()?;
        self.settings(&text, changed)
    }

    fn lock(&self) -> MutexGuard<'_, ()> {
        self.writing.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Runs `step` on a blocking thread, since it reads or writes files.
    fn blocking<R, E>(
        &self,
        step: impl FnOnce(&Self) -> Result<R, E> + Send + 'static,
    ) -> BoxFuture<'static, Result<R, E>>
    where
        R: Send + 'static,
        E: From<EntryStoreError> + Send + 'static,
    {
        let settings = self.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || step(&settings))
                .await
                .map_err(|error| E::from(EntryStoreError::new(error)))?
        })
    }
}

impl SettingsStore for SiteSettings {
    fn read(&self) -> BoxFuture<'_, Result<AdminSettings, EntryStoreError>> {
        self.blocking(Self::read_now)
    }

    fn change(
        &self,
        change: SettingsChange,
    ) -> BoxFuture<'_, Result<AdminSettings, SettingsError>> {
        self.blocking(move |settings| settings.change_now(change))
    }
}

/// The refusal a config error answers with, naming its field.
fn refused(error: ConfigError) -> SettingsError {
    match error {
        ConfigError::Field { field, problem, .. } => SettingsError::Refused { field, problem },
        ConfigError::Parse { field, source, .. } => SettingsError::Refused {
            field,
            problem: source.message().to_owned(),
        },
        error => SettingsError::Store(EntryStoreError::new(error)),
    }
}

/// Sets `key` to `new`, keeping the comments and spacing around the old
/// value, such as the comment block above the first key.
fn set(document: &mut DocumentMut, key: &str, new: Value) {
    match document.get_mut(key).and_then(Item::as_value_mut) {
        Some(old) => {
            let decor = old.decor().clone();
            *old = new;
            *old.decor_mut() = decor;
        }
        None => {
            document.insert(key, Item::Value(new));
        }
    }
}
