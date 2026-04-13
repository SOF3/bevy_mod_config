#![allow(clippy::multiple_bound_locations, reason = "intentionally test different syntaxes")]

use bevy_mod_config::{Config, ConfigField};

#[derive(Config)]
pub struct NamedStruct<T1, T2: Copy>
where
    T1: Clone + ConfigField,
    T2: ConfigField,
{
    pub v1: T1,
    pub v2: T2,
}

#[derive(Config)]
pub struct TupleStruct<T1, T2: Copy>(pub T1, pub T2)
where
    T1: Clone + ConfigField,
    T2: ConfigField;

#[derive(Config)]
pub enum Enum<T1, T2: Copy>
where
    T1: Clone + ConfigField,
    T2: ConfigField,
{
    Unit,
    Named { v1: T1, v2: T2 },
    Tuple(T1, T2),
}
