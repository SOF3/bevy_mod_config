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
/// The type of all fields must implement [`ConfigField`](crate::ConfigField).
/// This may be a [scalar type](crate::impl_scalar_config_field)
/// or another `#[derive(Config)]` type.
///
/// [Metadata](crate::ConfigField::Metadata) for each field may be specified
/// in the form `#[config(path = value_expr, ...)]`:
///
/// # Field-level attributes
///
/// Field-level attributes are in the form `#[config(path = value_expr, ...)]`.
/// Each `path = value_expr` is equivalent to `metadata.path = value_expr;`,
/// where `metadata` is the [`Metadata`](crate::ConfigField::Metadata) for this field.
/// `path` may be `.`-separated to address nested metadata fields.
/// Multiple assignments may be listed in the same attribute, separated by commas.
///
/// ## Scalar field types
///
/// For types that implement [`ConfigField`](crate::ConfigField) directly,
/// the metadata fields depend on the type.
/// For example, [`NumericMetadata`](crate::impls::NumericMetadata) exposes
/// `default`, `min`, `max`, `precision` and `slider`,
/// so they can be assigned like the following:
///
/// ```
/// #[derive(bevy_mod_config::Config)]
/// struct Settings {
///     #[config(default = 10, min = 0, max = 100)]
///     volume: u32,
/// }
/// ```
///
/// See the "Implementors" section in [`ConfigField`] documentation
/// to see the metadata of supported scalar types.
///
/// ## `#[derive(Config)]` struct types as fields
///
/// For fields whose type is a `#[derive(Config)]` struct,
/// the generated metadata struct mirrors the original struct:
/// each field holds the [`Metadata`](crate::ConfigField::Metadata) of the corresponding type.
/// Metadata can therefore be addressed by chaining field names with `.`.
///
/// For example, given:
///
/// ```
/// # use bevy_mod_config::Config;
/// #[derive(Config)]
/// struct Resolution {
///     width:  u32,
///     height: u32,
/// }
/// ```
///
/// ```
/// # use bevy_mod_config::Config;
/// # #[derive(Config)]
/// # struct Resolution { width: u32, height: u32 }
/// #[derive(Config)]
/// struct Settings {
///     #[config(width.default = 1920, height.default = 1080)]
///     resolution: Resolution,
/// }
/// ```
///
/// `width.default` first locates the metadata for `width: u32`,
/// which is a [`NumericMetadata<u32>`](crate::impls::NumericMetadata),
/// then updates the `default` field in the `NumericMetadata<u32>` struct to `1920`.
///
/// This may be recursively applied to nested structs/enums.
/// Default in the outer struct will override the default in the inner struct.
///
/// ## `#[derive(Config)]` enum types as fields
///
/// ### Configuring discriminant
///
/// When unspecified, the default variant of an enum is the first variant in declaration order,
/// or overridden by the container-level `#[config(discrim(...))]` attribute if present.
///
/// This can be further overridden at usage site with the metadata update syntax:
///
/// ```
/// # use bevy_mod_config::Config;
/// #[derive(Config)]
/// #[config(expose(discrim))] // we need to expose this to use `ThemeDiscrim` below
/// enum Theme {
///     Light,
///     Dark,
/// }
///
/// #[derive(Config)]
/// struct Settings {
///     // Override the default from `Light` to `Dark`.
///     #[config(discrim.default = ThemeDiscrim::Dark)]
///     theme: Theme,
/// }
/// ```
///
/// The `discrim` metadata field of derived enums is an
/// [`EnumDiscriminantMetadata`](crate::EnumDiscriminantMetadata);
/// `discrim.default` accesses [`EnuMDiscriminantMetadata::default`] to set the default variant.
///
/// ### Configuring variant fields
///
/// To avoid name collision, variant fields always start with `v_` followed by the variant name.
/// Further field accesses treat the enum variant fields as if a struct.
///
/// ```
/// # use bevy_mod_config::Config;
/// #[derive(Config)]
/// #[config(expose(discrim))]
/// enum Color {
///     White,
///     Rgb(u8, u8, u8),
///     Oklab { lightness: f32, a: f32, b: f32 },
/// }
///
/// #[derive(Config)]
/// struct Settings {
///     // Default to the `Rgb` variant with value (255, 128, 0).
///     #[config(
///         discrim.default = ColorDiscrim::Rgb,
///         v_Rgb.0.default = 255,
///         v_Rgb.1.default = 128,
///         v_Rgb.2.default = 0,
///     )]
///     foreground: Color,
///
///     // Default to the `Oklab` variant with value { l: 0.5, a: 0.0, b: 0.0 }.
///     #[config(
///         discrim.default = ColorDiscrim::Oklab,
///         v_Oklab.lightness.default = 0.5,
///     )]
///     background: Color,
/// }
/// ```
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
/// This type is mostly used to interact with field managers as a generic parameter;
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
/// Specifies the default [metadata](crate::EnumDiscriminantMetadata) for the enum discriminant.
///
/// This can be overridden at usage fields with `#[config(discrim.xxx = value_expr)]` on the field.
pub use bevy_mod_config_macros::Config;
