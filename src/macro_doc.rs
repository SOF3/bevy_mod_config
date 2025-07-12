/// Derives [`ConfigField`](crate::ConfigField) on a struct or enum,
/// allowing it to be embedded in another `ConfigField` struct/enum
/// or passed into [`init_config`](crate::AppExt::init_config).
///
/// For structs, this generates a field tree where
/// each field is a child node of the node for this struct.
///
/// For enums, this generates an enum discriminant node
/// indicating which variant is the current active one,
/// and a subtree for each variant where the variant is the subtree root.
///
/// The type of all fields must implement [`ConfigField`](crate::ConfigField),
/// which may be [scalar types](crate::impl_scalar_config_field)
/// or other `#[derie(Config)]` types.
///
/// [Metadata](crate::ConfigField::Metadata) for each field may be specified
/// in the form `#[config(field.path = value_expr, ...)]`:
/// ```
/// #[derive(bevy_mod_config::Config)]
/// struct Settings {
///     #[config(default = 10, min = Some(0), max = Some(100))]
///     volume: u32,
/// }
/// ```
///
/// Each `field.path = value_expr` is equivalent to setting `metadata.field.path = value_expr;`
/// on the metadata struct passed for the field.
/// By convention, the `default` field on each metadata struct specifies the default value.
/// See the documentation of the corresponding metadata types for the available fields.
///
/// # Container-level attributes
/// ## `#[config(expose)]`
/// `#[derive(Config)]` generates additional types to be used in accessor code.
/// By default, these types are hidden in a `const _: () = {...};` block
/// to avoid polluting the user namespace.
/// However, it may be desirable to reference these types under certain conditions,
/// e.g. for enum matching, naming parameter types, etc.
/// `#[config(expose)]` exposes all such types,
/// while `#[config(expose(xxx))]` exposes only the `xxx` structs:
///
/// ### `#[config(expose(read))]`
/// Exposes the [`Reader`](crate::ConfigField::Reader) type.
/// This is the type returned by [`ReadConfig::read`](crate::ReadConfig::read),
/// where each field corresponds to the `Reader` type of the field type in the input.
///
/// The default identifier is `{InputIdent}Read`.
/// This can be renamed with `#[config(expose(read = NewIdent))]`.
///
/// ### `#[config(expose(changed))]`
/// Exposes the [`Changed`](crate::ConfigField::Changed) type.
/// This is the type returned by [`ReadConfig::changed`](crate::ReadConfig::changed).
///
/// The default identifier is `{InputIdent}Changed`.
/// This can be renamed with `#[config(expose(changed = NewIdent))]`.
///
/// ### `#[config(expose(discrim))]`
/// Exposes the enum discriminant type.
/// Must only be used on enum types.
/// This is type is mostly used to interact with field managers as a generic parameter;
/// it is unusual to use this type in user code.
///
/// The default identifier is `{InputIdent}Discrim`.
/// This can be renamed with `#[config(expose(discrim = NewIdent))]`.
///
/// ### `#[config(expose(spawn_handle))]`
/// Exposes the spawn handle type containing the entity IDs of the config field tree.
/// Must only be used on enum types.
/// This is type is mostly used by the derived trait internally;
/// it is unusual to use this type in user code.
///
/// The default identifier is `{InputIdent}Spawnhandle`.
/// This can be renamed with `#[config(expose(spawn_handle = NewIdent))]`.
///
/// ## `#[config(crate_path(::path::to::bevy_mod_config))]`
/// Overrides the path to the `bevy_mod_config` crate.
/// The default is `::bevy_mod_config`.
/// This is mostly only useful if the library name of `bevy_mod_config` was renamed in Cargo.toml.
///
/// ## `#[config(discrim(...))]`
/// Specifies the [metadata](crate::EnumDiscriminantMetadata) for the enum discriminant.
pub use bevy_mod_config_macros::Config;
