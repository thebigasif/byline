//! Where HTTPS keeps its files: the account key and the certificate with
//! its key, in `data/https/` in the site folder (RULES.md BL-FS-2).

use std::io;
use std::path::{Path, PathBuf};

use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::{AccountKey, Certificate, HttpsError, Issued};

/// The account key's file name.
const ACCOUNT: &str = "account.json";

/// The certificate chain's file name.
const CHAIN: &str = "certificate.pem";

/// The certificate key's file name.
const KEY: &str = "key.pem";

/// Who may read and write the files: the owner alone, because two of them
/// are secrets.
#[cfg(unix)]
const OWNER_ONLY: u32 = 0o600;

/// The folder HTTPS keeps its files in. Each file is written whole under a
/// new name, then moved into place, so a crash never leaves half a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpsFolder {
    folder: PathBuf,
}

impl HttpsFolder {
    /// The files in `folder`, which is made when the first file is saved.
    pub fn new(folder: PathBuf) -> Self {
        Self { folder }
    }

    /// The saved account key, if there is one.
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::Folder`] when the file cannot be read, and
    /// [`HttpsError::AccountFile`] when it is not an account key.
    pub async fn account(&self) -> Result<Option<AccountKey>, HttpsError> {
        match self.read(ACCOUNT).await? {
            Some(json) => AccountKey::from_json(&json).map(Some),
            None => Ok(None),
        }
    }

    /// Saves the account key, readable by the owner alone.
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::Folder`] when the file cannot be written.
    pub async fn save_account(&self, key: &AccountKey) -> Result<(), HttpsError> {
        self.write(ACCOUNT, &key.to_json()?).await
    }

    /// The saved certificate, if both of its files are there.
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::Folder`] when a file cannot be read, and the
    /// errors of [`Certificate::from_pem`] when they do not hold one.
    pub async fn certificate(&self) -> Result<Option<Certificate>, HttpsError> {
        let (Some(chain), Some(key)) = (self.read(CHAIN).await?, self.read(KEY).await?) else {
            return Ok(None);
        };
        Certificate::from_pem(&chain, &key).map(Some)
    }

    /// Checks a new certificate, then saves it, its key readable by the
    /// owner alone.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Certificate::from_pem`] before anything is
    /// written, and [`HttpsError::Folder`] when a file cannot be written.
    pub async fn save_certificate(&self, issued: &Issued) -> Result<Certificate, HttpsError> {
        let certificate = Certificate::from_pem(issued.chain.as_bytes(), issued.key.as_bytes())?;
        self.write(KEY, issued.key.as_bytes()).await?;
        self.write(CHAIN, issued.chain.as_bytes()).await?;
        Ok(certificate)
    }

    /// The path of the file `name` in the folder.
    fn path(&self, name: &str) -> PathBuf {
        let mut path = self.folder.clone();
        path.push(name);
        path
    }

    async fn read(&self, name: &str) -> Result<Option<Vec<u8>>, HttpsError> {
        let path = self.path(name);
        match fs::read(&path).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(HttpsError::Folder { path, source }),
        }
    }

    async fn write(&self, name: &str, bytes: &[u8]) -> Result<(), HttpsError> {
        let failed = |path: &Path| {
            let path = path.to_path_buf();
            move |source| HttpsError::Folder { path, source }
        };
        fs::create_dir_all(&self.folder)
            .await
            .map_err(failed(&self.folder))?;
        let path = self.path(name);
        let new = self.path(&format!("{name}.new"));
        // A file left by a crash may be readable by others, and opening
        // it would keep its permissions, so it goes first.
        if let Err(error) = fs::remove_file(&new).await
            && error.kind() != io::ErrorKind::NotFound
        {
            return Err(failed(&new)(error));
        }
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        options.mode(OWNER_ONLY);
        let mut file = options.open(&new).await.map_err(failed(&new))?;
        file.write_all(bytes).await.map_err(failed(&new))?;
        file.sync_all().await.map_err(failed(&new))?;
        fs::rename(&new, &path).await.map_err(failed(&path))
    }
}
