use bevy_ecs::entity::Entity;
use bevy_ecs::query::{QueryData, QueryFilter};
use bevy_ecs::system::Query;

/// A [`Query`]-like type that extracts component data from an entity,
/// allowing a large query to be mapped to a subset of its requests.
pub trait QueryLike: Copy {
    /// The data returned for each entity in the query.
    ///
    /// To avoid interfering with other entities that may not have a certain component,
    /// `Item` should consist of `Option` components instead of directly requesting components.
    type Item;

    /// Returns the data for the given entity, if it exists.
    ///
    /// Returns `None` if the entity does not match the *root* query used in the system.
    fn get(self, entity: Entity) -> Option<Self::Item>;

    /// Maps the query result for each entity to another type.
    fn map<F, R>(self, map_fn: F) -> impl QueryLike<Item = R>
    where
        F: Fn(Self::Item) -> R + Copy,
    {
        MappedQuery { base: self, map_fn }
    }
}

impl<'query, D: QueryData, F: QueryFilter> QueryLike for &'query Query<'_, '_, D, F> {
    type Item = <D::ReadOnly as QueryData>::Item<'query>;

    fn get(self, entity: Entity) -> Option<Self::Item> { self.get(entity).ok() }
}

/// Used to implement [`QueryLike::map`].
#[derive(Clone, Copy)]
struct MappedQuery<Q, F> {
    base:   Q,
    map_fn: F,
}

impl<Q, F, R> QueryLike for MappedQuery<Q, F>
where
    Q: QueryLike,
    F: Fn(Q::Item) -> R + Copy,
{
    type Item = R;

    fn get(self, entity: Entity) -> Option<R> { self.base.get(entity).map(self.map_fn) }
}
