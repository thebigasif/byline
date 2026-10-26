//! Published content in memory: what `khalid serve` reads from the index
//! after each change, and what tests fill by hand.

use std::cmp::Ordering;

use super::{BoxFuture, ContentStore, ContentStoreError, Listed, PostKey, SiteResources};
use crate::pagination::Meta;
use crate::query::{Direction, FieldValues, Limit, ListQuery, Literal};
use crate::resources::{Collection, Post};

/// Entries held in memory, filtered, sorted and paged the way a database
/// store would.
#[derive(Debug, Clone, Default)]
pub struct MemoryContentStore {
    entries: Vec<(Collection, Post)>,
    site: SiteResources,
}

impl MemoryContentStore {
    /// A store holding `entries`, each in its collection, and no authors.
    pub fn new(entries: Vec<(Collection, Post)>) -> Self {
        let site = SiteResources::default();
        Self { entries, site }
    }

    /// The same store with the site's small lists, such as its authors.
    #[must_use]
    pub fn with_site(self, site: SiteResources) -> Self {
        Self { site, ..self }
    }

    fn in_collection(&self, collection: Collection) -> impl Iterator<Item = &Post> {
        self.entries
            .iter()
            .filter(move |(of, _)| *of == collection)
            .map(|(_, post)| post)
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
        let entries = self.in_collection(collection);
        Box::pin(std::future::ready(Ok(select(entries, query))))
    }

    fn find<'a>(
        &'a self,
        collection: Collection,
        key: &'a PostKey,
    ) -> BoxFuture<'a, Result<Option<Post>, ContentStoreError>> {
        Box::pin(std::future::ready(Ok(self.lookup(collection, key))))
    }

    fn site(&self) -> BoxFuture<'_, Result<SiteResources, ContentStoreError>> {
        Box::pin(std::future::ready(Ok(self.site.clone())))
    }
}

/// One page of the `items` that match `query`'s filter, in its order. With
/// no order, the newest `published_at` comes first, and items without one
/// keep the order they came in.
pub(crate) fn select<'a, T: FieldValues + Clone + 'a>(
    items: impl Iterator<Item = &'a T>,
    query: &ListQuery,
) -> Listed<T> {
    let mut found: Vec<&T> = items
        .filter(|item| query.filter().is_none_or(|filter| filter.matches(*item)))
        .collect();
    found.sort_by(|a, b| {
        if query.order().is_empty() {
            return first(*b, "published_at").cmp_with(&first(*a, "published_at"));
        }
        query.order().iter().fold(Ordering::Equal, |sofar, key| {
            sofar.then_with(|| {
                let order = first(*a, key.field()).cmp_with(&first(*b, key.field()));
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

/// The first value of `field`, which a sort uses.
fn first(item: &impl FieldValues, field: &str) -> SortValue {
    SortValue(item.values(field).into_iter().next())
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
