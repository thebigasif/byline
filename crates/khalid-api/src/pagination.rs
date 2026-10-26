//! `meta.pagination`, which every list carries.

use serde::{Serialize, Serializer};
use utoipa::ToSchema;
use utoipa::openapi::schema::{ObjectBuilder, OneOfBuilder, Schema, Type};

use crate::query::Limit;

/// The `meta` object beside every list (`docs/standards/api.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Meta {
    pagination: Pagination,
}

impl Meta {
    /// The meta for `page` of a list of `total` items, `limit` per page.
    pub fn new(page: u32, limit: Limit, total: u64) -> Self {
        Self {
            pagination: Pagination::new(page, limit, total),
        }
    }

    /// The pagination.
    pub fn pagination(&self) -> &Pagination {
        &self.pagination
    }
}

/// Where a page sits in its list, in Ghost's shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Pagination {
    /// This page, from 1.
    page: u32,
    /// Items per page, or `all`.
    #[serde(serialize_with = "limit_value")]
    #[schema(schema_with = limit_schema)]
    limit: Limit,
    /// How many pages the list has, at least 1.
    pages: u64,
    /// How many items the list has.
    total: u64,
    /// The next page, if there is one.
    next: Option<u64>,
    /// The previous page, if there is one.
    prev: Option<u64>,
}

impl Pagination {
    fn new(page: u32, limit: Limit, total: u64) -> Self {
        let pages = match limit {
            Limit::Count(count) => total.div_ceil(u64::from(count)).max(1),
            Limit::All => 1,
        };
        let current = u64::from(page);
        Self {
            page,
            limit,
            pages,
            total,
            next: (current < pages).then(|| current + 1),
            prev: current.checked_sub(1).filter(|prev| *prev > 0),
        }
    }

    /// How many items come before this page, for the store to skip.
    pub fn offset(&self) -> u64 {
        match self.limit {
            Limit::Count(count) => u64::from(self.page.saturating_sub(1)) * u64::from(count),
            Limit::All => 0,
        }
    }

    /// Items per page.
    pub fn limit(&self) -> Limit {
        self.limit
    }
}

/// The schema of `limit`: a whole number from 1 to 100, or `"all"`.
fn limit_schema() -> Schema {
    let count = ObjectBuilder::new()
        .schema_type(Type::Integer)
        .minimum(Some(1))
        .maximum(Some(100));
    let all = ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some(["all"]));
    OneOfBuilder::new().item(count).item(all).into()
}

/// Writes a count as a number and `all` as the string `"all"`.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's serialize_with passes the field by reference"
)]
fn limit_value<S: Serializer>(limit: &Limit, serializer: S) -> Result<S::Ok, S::Error> {
    match limit {
        Limit::Count(count) => serializer.serialize_u32(*count),
        Limit::All => serializer.serialize_str("all"),
    }
}
