use bevy_ecs::entity::Entity;
use bevy_ecs::query::{QueryData, QueryFilter};
use bevy_ecs::system::Query;

pub trait QueryLike: Copy {
    type Item;

    fn get(self, entity: Entity) -> Option<Self::Item>;

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

#[derive(Clone, Copy)]
pub struct MappedQuery<Q, F> {
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
