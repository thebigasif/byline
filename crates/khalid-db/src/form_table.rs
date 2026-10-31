//! Form answers on SQLite, for `khalid-forms`.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_forms::{FormId, FormStore, FormStoreError, Submission};
use sqlx::Row;

use crate::{Database, DbError, SiteId};

/// One site's form answers in `form_submissions`, a visitor table a
/// rebuild never touches. Each row's `fields` is a JSON array of
/// `{"name", "value"}` objects in the order the form sent them. Every
/// query is bound to the site. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct FormTable {
    db: Database,
    site: SiteId,
}

impl FormTable {
    /// The form answers of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    /// Every answer kept for `form`, oldest first, each with when it was
    /// sent.
    ///
    /// # Errors
    ///
    /// [`DbError::Query`] when the database cannot be read, and
    /// [`DbError::Unreadable`] when a row holds what this table never
    /// writes.
    pub async fn of_form(
        &self,
        form: &FormId,
    ) -> Result<Vec<(Submission, DateTime<Utc>)>, DbError> {
        let rows = sqlx::query(
            "SELECT s.id, s.page_path, s.created_at, \
             json_extract(f.value, '$.name') AS name, json_extract(f.value, '$.value') AS value \
             FROM form_submissions AS s, json_each(s.fields) AS f \
             WHERE s.site_id = ?1 AND s.form = ?2 AND s.status = 'received' \
             ORDER BY s.created_at, s.id, f.key",
        )
        .bind(self.site.as_row())
        .bind(form.as_str())
        .fetch_all(&self.db.readers)
        .await?;
        let mut kept: Vec<(i64, Submission, DateTime<Utc>)> = Vec::new();
        for row in &rows {
            let id: i64 = row.try_get("id")?;
            let field = (row.try_get("name")?, row.try_get("value")?);
            if let Some((last, submission, _)) = kept.last_mut()
                && *last == id
            {
                submission.fields.push(field);
                continue;
            }
            let page: Option<String> = row.try_get("page_path")?;
            let created_at: String = row.try_get("created_at")?;
            let at = DateTime::parse_from_rfc3339(&created_at)
                .map(|time| time.with_timezone(&Utc))
                .map_err(|_| DbError::Unreadable {
                    column: "created_at",
                    value: created_at.clone(),
                })?;
            let submission = Submission {
                form: form.clone(),
                page: page.unwrap_or_default(),
                fields: vec![field],
            };
            kept.push((id, submission, at));
        }
        Ok(kept.into_iter().map(|(_, s, at)| (s, at)).collect())
    }

    async fn insert_now(&self, submission: &Submission, at: DateTime<Utc>) -> Result<(), DbError> {
        let mut tx = self.db.writer.begin().await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO form_submissions (site_id, form, page_path, fields, status, created_at) \
             VALUES (?1, ?2, ?3, '[]', 'received', ?4) RETURNING id",
        )
        .bind(self.site.as_row())
        .bind(submission.form.as_str())
        .bind(&submission.page)
        .bind(at.to_rfc3339_opts(SecondsFormat::Micros, true))
        .fetch_one(&mut *tx)
        .await?;
        for (name, value) in &submission.fields {
            sqlx::query(
                "UPDATE form_submissions \
                 SET fields = json_insert(fields, '$[#]', json_object('name', ?1, 'value', ?2)) \
                 WHERE id = ?3",
            )
            .bind(name)
            .bind(value)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

impl FormStore for FormTable {
    async fn insert(
        &self,
        submission: &Submission,
        at: DateTime<Utc>,
    ) -> Result<(), FormStoreError> {
        self.insert_now(submission, at)
            .await
            .map_err(|error| FormStoreError::Write {
                source: Box::new(error),
            })
    }
}
