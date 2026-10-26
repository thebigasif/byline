//! Published content in memory, for tests.

use std::cmp::Ordering;

use super::{BoxFuture, ContentStore, ContentStoreError, Listed, PostKey};
use crate::pagination::Meta;
use crate::query::{Direction, FieldValues, Limit, ListQuery, Literal};
use crate::resources::{Collection, Post};

/// Entries held in memory, filtered, sorted and paged the way a database
/// store would. For tests only.
#[derive(Debug, Clone, Default)]
pub struct MemoryContentStore {
    entries: Vec<(Collection, Post)>,
}

impl MemoryContentStore {
    /// A store holding `entries`, each in its collection.
    pub fn new(entries: Vec<(Collection, Post)>) -> Self {
        Self { entries }
    }

    fn in_collection(&self, collection: Collection) -> impl Iterator<Item = &Post> {
        self.entries
            .iter()
            .filter(move |(of, _)| *of == collection)
            .map(|(_, post)| post)
    }

    fn select(&self, collection: Collection, query: &ListQuery) -> Listed<Post> {
        let mut found: Vec<&Post> = self
            .in_collection(collection)
            .filter(|post| query.filter().is_none_or(|filter| filter.matches(*post)))
            .collect();
        found.sort_by(|a, b| {
            if query.order().is_empty() {
                return first(b, "published_at").cmp_with(&first(a, "published_at"));
            }
            query.order().iter().fold(Ordering::Equal, |sofar, key| {
                sofar.then_with(|| {
                    let order = first(a, key.field()).cmp_with(&first(b, key.field()));
                    match key.direction() {
                        Direction::Ascending => order,
                        Direction::Descending => order.reverse(),
                    }
                })
            })
        });
        let total = u64::try_from(found.len()).unwrap_or(u64::MAX);
        let meta = Meta::new(query.page(), query.limit(), total);
        let skip = usize::try_from(meta.pagination().offset()).unwrap_or(usize::MAX);
        let take = match query.limit() {
            Limit::Count(count) => usize::try_from(count).unwrap_or(usize::MAX),
            Limit::All => usize::MAX,
        };
        let items = found.into_iter().skip(skip).take(take).cloned().collect();
        Listed::new(items, total)
    }

    fn lookup(&self, collection: Collection, key: &PostKey) -> Option<Post> {
        self.in_collection(collection)
            .find(|post| match key {
                PostKey::Id(id) => post.id() == id,
                PostKey::Slug(slug) => post.slug() == slug,
            })
            .cloned()
    }
}

/// Memory answers at once, so each future is ready when returned.
impl ContentStore for MemoryContentStore {
    fn list<'a>(
        &'a self,
        collection: Collection,
        query: &'a ListQuery,
    ) -> BoxFuture<'a, Result<Listed<Post>, ContentStoreError>> {
        Box::pin(std::future::ready(Ok(self.select(collection, query))))
    }

    fn find<'a>(
        &'a self,
        collection: Collection,
        key: &'a PostKey,
    ) -> BoxFuture<'a, Result<Option<Post>, ContentStoreError>> {
        Box::pin(std::future::ready(Ok(self.lookup(collection, key))))
    }
}

/// The first value of `field`, which a sort uses.
fn first(post: &Post, field: &str) -> SortValue {
    SortValue(post.values(field).into_iter().next())
}

/// A value to sort by. A missing value sorts first, and values of
/// different kinds keep their order.
struct SortValue(Option<Literal>);

impl SortValue {
    fn cmp_with(&self, other: &Self) -> Ordering {
        match (&self.0, &other.0) {
            (Some(Literal::Text(a)), Some(Literal::Text(b))) => a.cmp(b),
            (Some(Literal::Integer(a)), Some(Literal::Integer(b))) => a.cmp(b),
            (Some(Literal::Bool(a)), Some(Literal::Bool(b))) => a.cmp(b),
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }
}
