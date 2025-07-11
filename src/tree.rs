use alloc::string::String;
use alloc::vec::Vec;
use core::ops;

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::EntityRef;

use crate::FieldGeneration;

/// Marks an entity as a config field node.
#[derive(Component)]
pub struct ConfigNode {
    /// Context information passed to
    /// [`ConfigFieldFor::spawn_world`](super::ConfigFieldFor::spawn_world).
    pub path:       Vec<String>,
    /// The generation of a field, used for change detection.
    pub generation: FieldGeneration,
}

/// Marks an entity as a root config node.
#[derive(Component)]
pub struct RootNode;

/// Marks an entity as a child node of a config field.
///
/// This is a relationship component.
#[derive(Component)]
#[relationship(relationship_target = ChildNodeList)]
pub struct ChildNodeOf(pub Entity);

/// Lists the child nodes of a config field node.
///
/// This can be used by managers to handle config fields hierarchically.
#[derive(Component)]
#[relationship_target(relationship = ChildNodeOf)]
pub struct ChildNodeList(Vec<Entity>);

impl ops::Deref for ChildNodeList {
    type Target = [Entity];

    fn deref(&self) -> &Self::Target { &self.0 }
}

/// Marks an entity as a scalar config field.
#[derive(Component)]
pub struct ScalarField;

/// If a node entity has this component,
/// it is conditionally "irrelevant" based on the state of another entity.
///
/// Irrelevance means that the node does not play an active role in the current world,
/// such as the inactive variants of an enum config tree.
///
/// Relevance is not inserted into descendant nodes automatically;
/// [`SpawnContext::join`](crate::SpawnContext::join) always returns an empty dependency.
/// Managers that depend on node relevance are expected to traverse the ancestors
/// to resolve whether the node belongs to an irrelevant subtree.
#[derive(Component, Clone)]
pub struct ConditionalRelevance {
    /// The entity that this node depends on for its relevance.
    pub dependency:         Entity,
    /// Tests whether a dependency entity is relevant with its current value.
    pub is_entity_relevant: fn(EntityRef) -> bool,
}
