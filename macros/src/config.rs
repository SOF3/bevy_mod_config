use std::iter;

use either::Either;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

pub fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    fn ifelse_tuple<T>(left: bool, value: T) -> (Option<T>, Option<T>) {
        if left { (Some(value), None) } else { (None, Some(value)) }
    }

    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let mut item_attrs = ItemAttrs::default();
    for attr in input.attrs.iter().filter(|attr| attr.path().is_ident("config")) {
        let parsed: ItemAttrParse = attr.parse_args()?;
        for item in parsed.items {
            item.apply(&mut item_attrs);
        }
    }
    let idents = Idents::new(&input)?;
    let input = Input::new(&input, &item_attrs, &idents)?;

    let spawn_handle = gen_spawn_handle(&item_attrs.crate_path, &idents, &input);
    let read = gen_read(&item_attrs.crate_path, &idents, &input);
    let changed = gen_changed(&item_attrs.crate_path, &idents, &input);
    let discrim = gen_discrim(&item_attrs.crate_path, &idents, &input);
    let impl_config_field = gen_impl_config_field(&item_attrs.crate_path, &idents, &input);

    let (spawn_handle_expose, spawn_handle_hidden) =
        ifelse_tuple(item_attrs.expose_spawn_handle, spawn_handle);
    let (read_expose, read_hidden) = ifelse_tuple(item_attrs.expose_read, read);
    let (changed_expose, changed_hidden) = ifelse_tuple(item_attrs.expose_changed, changed);
    let (discrim_expose, discrim_hidden) = ifelse_tuple(item_attrs.expose_discrim, discrim);

    let dead_code_workaround = dead_code_workaround(&input);

    let output = quote! {
        #spawn_handle_expose
        #read_expose
        #changed_expose
        #discrim_expose
        const _: () = {
            #spawn_handle_hidden
            #read_hidden
            #changed_hidden
            #discrim_hidden
            #impl_config_field

            #dead_code_workaround
        };
    };
    if item_attrs.debug_print {
        println!("#[derive(Config)] output:\n{output}");
    }
    Ok(output)
}

fn gen_spawn_handle(crate_path: &syn::Path, idents: &Idents, input: &Input) -> TokenStream {
    let vis = input.vis;
    let spawn_fields = input.data.iter_field_data().map(|field| {
        let field_ident = &field.spawn_handle_field;
        let field_ty = &field.ty;
        quote! {
            #field_ident: <#field_ty as #crate_path::ConfigField>::SpawnHandle,
        }
    });
    let spawn_handle_ident = &idents.spawn_handle_ident;

    quote! {
        #[allow(non_snake_case)]
        #vis struct #spawn_handle_ident {
            #(#spawn_fields)*
        }
    }
}

fn gen_read(crate_path: &syn::Path, idents: &Idents, input: &Input) -> TokenStream {
    match input.data {
        InputData::Struct(ref struct_input) => {
            gen_read_struct(crate_path, input.vis, idents, struct_input)
        }
        InputData::Enum(ref enum_input) => gen_read_enum(crate_path, input.vis, idents, enum_input),
    }
}

fn gen_read_struct(
    crate_path: &syn::Path,
    vis: &syn::Visibility,
    idents: &Idents,
    input: &StructInput,
) -> TokenStream {
    let read_ident = &idents.read_ident;

    if input.named_fields {
        let read_fields = input.fields.iter().map(|field| {
            let field_vis = field.vis;
            let field_ident = field.ident.ident().expect("named_fields implies Ident");
            let field_ty = field.data.ty;
            quote! {
                #field_vis #field_ident: <#field_ty as #crate_path::ConfigField>::Reader<'a>,
            }
        });
        quote! {
            #vis struct #read_ident<'a> {
                #(#read_fields)*
            }
        }
    } else {
        let read_fields = input.fields.iter().map(|field| {
            let field_ty = &field.data.ty;
            quote! {
                <#field_ty as #crate_path::ConfigField>::Reader<'a>,
            }
        });
        quote! {
            #vis struct #read_ident<'a> (
                #(#read_fields)*
            );
        }
    }
}

fn gen_read_enum(
    crate_path: &syn::Path,
    vis: &syn::Visibility,
    idents: &Idents,
    input: &EnumInput,
) -> TokenStream {
    let read_ident = &idents.read_ident;
    let read_variants = input.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        match variant.field_syntax {
            FieldSyntax::Named => {
                let read_fields = variant.fields.iter().map(|field| {
                    let field_ident = field.ident.ident().expect("named_fields implies Ident");
                    let field_ty = &field.data.ty;
                    quote! {
                        #field_ident: <#field_ty as #crate_path::ConfigField>::Reader<'a>,
                    }
                });
                quote! {
                    #variant_ident { #(#read_fields)* }
                }
            }
            FieldSyntax::Unnamed => {
                let read_fields = variant.fields.iter().map(|field| {
                    let field_ty = &field.data.ty;
                    quote! {
                        <#field_ty as #crate_path::ConfigField>::Reader<'a>,
                    }
                });
                quote! {
                    #variant_ident(#(#read_fields)*)
                }
            }
            FieldSyntax::Unit => quote!(#variant_ident),
        }
    });
    quote! {
        #vis enum #read_ident<'a> {
            #(#read_variants,)*
        }
    }
}

fn gen_changed(crate_path: &syn::Path, idents: &Idents, input: &Input) -> TokenStream {
    match input.data {
        InputData::Struct(ref struct_input) => {
            gen_changed_struct(crate_path, input.vis, idents, struct_input)
        }
        InputData::Enum(ref enum_input) => {
            gen_changed_enum(crate_path, input.vis, idents, enum_input)
        }
    }
}

fn gen_changed_struct(
    crate_path: &syn::Path,
    vis: &syn::Visibility,
    idents: &Idents,
    input: &StructInput,
) -> TokenStream {
    let changed_ident = &idents.changed_ident;

    if input.named_fields {
        let changed_fields = input.fields.iter().map(|field| {
            let field_vis = field.vis;
            let field_ident = field.ident.ident().expect("named_fields implies Ident");
            let field_ty = field.data.ty;
            quote! {
                #field_vis #field_ident: <#field_ty as #crate_path::ConfigField>::Changed,
            }
        });
        let changed_derives = changed_derives(crate_path);
        quote! {
            #changed_derives
            #vis struct #changed_ident {
                #(#changed_fields)*
            }
        }
    } else {
        let changed_fields = input.fields.iter().map(|field| {
            let field_ty = field.data.ty;
            quote! {
                <#field_ty as #crate_path::ConfigField>::Changed,
            }
        });
        let changed_derives = changed_derives(crate_path);
        quote! {
            #changed_derives
            #vis struct #changed_ident (
                #(#changed_fields)*
            );
        }
    }
}

fn gen_changed_enum(
    crate_path: &syn::Path,
    vis: &syn::Visibility,
    idents: &Idents,
    input: &EnumInput,
) -> TokenStream {
    let changed_ident = &idents.changed_ident;
    let changed_variants = input.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        match variant.field_syntax {
            FieldSyntax::Named => {
                let changed_fields = variant.fields.iter().map(|field| {
                    let field_ident = field.ident.ident().expect("named_fields implies Ident");
                    let field_ty = &field.data.ty;
                    quote! {
                        #field_ident: <#field_ty as #crate_path::ConfigField>::Changed,
                    }
                });
                quote! {
                    #variant_ident { #(#changed_fields)* }
                }
            }
            FieldSyntax::Unnamed => {
                let changed_fields = variant.fields.iter().map(|field| {
                    let field_ty = &field.data.ty;
                    quote! {
                        <#field_ty as #crate_path::ConfigField>::Changed,
                    }
                });
                quote! {
                    #variant_ident(#(#changed_fields)*)
                }
            }
            FieldSyntax::Unit => quote!(#variant_ident),
        }
    });
    let changed_derives = changed_derives(crate_path);
    quote! {
        #changed_derives
        #vis enum #changed_ident {
            #(#changed_variants,)*
        }
    }
}

fn changed_derives(crate_path: &syn::Path) -> TokenStream {
    quote! {
        #[derive(
            #crate_path::__import::Clone,
            #crate_path::__import::PartialEq,
            #crate_path::__import::Eq,
        )]
    }
}

fn gen_discrim(crate_path: &syn::Path, idents: &Idents, input: &Input) -> TokenStream {
    let vis = input.vis;
    let InputData::Enum(ref enum_input) = input.data else {
        return quote! {};
    };
    let discrim_ident = idents.discrim_ident().expect("Enum must have a discriminant type");
    let variant_names = enum_input.variants.iter().map(|variant| variant.ident);
    let metadata_ident = format_ident!("{}Metadata", discrim_ident);

    let default_variant_name =
        enum_input.variants.first().expect("checked during Input::new").ident;

    let variants_const = enum_input.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        quote! { #discrim_ident::#variant_ident }
    });
    let into_usize_arms = enum_input.variants.iter().enumerate().map(|(index, variant)| {
        let variant_ident = &variant.ident;
        quote! {
            #discrim_ident::#variant_ident => #index,
        }
    });
    let name_arms = enum_input.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        quote! {
            #discrim_ident::#variant_ident => #crate_path::__import::stringify!(#variant_ident),
        }
    });
    let from_name_arms = enum_input.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        quote! {
            #crate_path::__import::stringify!(#variant_ident) =>
                #crate_path::__import::Some(#discrim_ident::#variant_ident),
        }
    });

    quote! {
        #[derive(
            #crate_path::__import::Debug,
            #crate_path::__import::Clone,
            #crate_path::__import::Copy,
            #crate_path::__import::PartialEq,
            #crate_path::__import::Eq,
        )]
        #vis enum #discrim_ident { #(#variant_names,)* }

        impl #crate_path::EnumDiscriminant for #discrim_ident {
            const VARIANTS: &'static [Self] = &[#(#variants_const),*];

            fn into_usize(self) -> usize {
                match self {
                    #(#into_usize_arms)*
                }
            }

            fn name(self) -> &'static str {
                match self {
                    #(#name_arms)*
                }
            }

            fn from_name(name: &str) -> #crate_path::__import::Option<Self> {
                match name {
                    #(#from_name_arms)*
                    _ => #crate_path::__import::None,
                }
            }
        }

        impl #crate_path::ConfigField for #discrim_ident {
            type SpawnHandle = #crate_path::__import::Entity;
            type Reader<'a> = #discrim_ident;
            type Metadata = #metadata_ident;
            type Changed = #crate_path::FieldGeneration;

            fn read_world<'a>(
                __config_query: &'a #crate_path::__import::Query<
                    #crate_path::__import::EntityRef,
                    #crate_path::__import::With<#crate_path::ConfigData>,
                >,
                __config_spawn_handle: &Self::SpawnHandle,
            ) -> Self::Reader<'a> {
                let entity = __config_query
                    .get(*__config_spawn_handle)
                    .expect("entity managed by config field must remain active as long as the config handle is used");
                let data = entity
                    .get::<#crate_path::ScalarData<#crate_path::EnumDiscriminantWrapper<#discrim_ident>>>()
                    .expect("entity must have been spawned with a ScalarData of the corresponding type");
                data.0.0
            }

            fn changed(
                __config_query: &#crate_path::__import::Query<(
                    &#crate_path::ConfigData,
                    #crate_path::__import::EntityRef,
                )>,
                &__config_spawn_handle: &Self::SpawnHandle,
            ) -> Self::Changed {
                let entity = __config_query
                    .get(__config_spawn_handle)
                    .expect("entity managed by config field must remain active as long as the config handle is used");
                entity.0.generation
            }
        }

        impl<__ConfigManager: #crate_path::Manager> #crate_path::ConfigFieldFor<__ConfigManager> for #discrim_ident
        where __ConfigManager: #crate_path::manager::Supports<#crate_path::EnumDiscriminantWrapper<#discrim_ident>> {
            fn spawn_world(
                __config_world: &mut #crate_path::__import::World,
                __config_ctx: #crate_path::SpawnContext,
                __config_metadata: &Self::Metadata,
            ) -> Self::SpawnHandle {
                let __config_manager_comp = __config_world
                    .resource_mut::<#crate_path::manager::Instance<__ConfigManager>>()
                    .new_entity::<#crate_path::EnumDiscriminantWrapper<#discrim_ident>>();
                __config_world.spawn((
                    #crate_path::ConfigData { ctx: __config_ctx, generation: #crate_path::__import::Default::default()  },
                    #crate_path::ScalarData(#crate_path::EnumDiscriminantWrapper(__config_metadata.default)),
                    __config_manager_comp,
                ))
                    .id()
            }
        }

        struct #metadata_ident {
            pub default: #discrim_ident,
        }

        impl #crate_path::__import::Default for #metadata_ident {
            fn default() -> Self {
                Self { default: #discrim_ident::#default_variant_name }
            }
        }
    }
}

fn gen_impl_config_field(crate_path: &syn::Path, idents: &Idents, input: &Input) -> TokenStream {
    let input_ident = &input.ident;
    let Idents { spawn_handle_ident, read_ident, changed_ident, .. } = idents;
    let spawn_world = gen_spawn_world(crate_path, idents, input);
    let read_world = gen_read_world(crate_path, idents, input);
    let changed_fn = gen_changed_fn(crate_path, idents, input);

    let where_clauses = input.data.iter_field_data().map(|field| {
        let field_ty = &field.ty;
        quote! {
            #field_ty: #crate_path::ConfigFieldFor<__ConfigManager>,
        }
    });

    quote! {
        impl #crate_path::ConfigField for #input_ident {
            type Reader<'a> = #read_ident<'a>;
            type SpawnHandle = #spawn_handle_ident;
            type Metadata = #crate_path::StructMetadata;
            type Changed = #changed_ident;

            fn read_world<'a>(
                __config_query: &'a #crate_path::__import::Query<
                    #crate_path::__import::EntityRef,
                    #crate_path::__import::With<#crate_path::ConfigData>,
                >,
                __config_spawn_handle: &Self::SpawnHandle,
            ) -> Self::Reader<'a> { #read_world }

            fn changed(
                __config_query: &#crate_path::__import::Query<(
                    &#crate_path::ConfigData,
                    #crate_path::__import::EntityRef,
                )>,
                __config_spawn_handle: &Self::SpawnHandle,
            ) -> Self::Changed { #changed_fn }
        }

        impl<__ConfigManager: #crate_path::Manager>
        #crate_path::ConfigFieldFor<__ConfigManager> for #input_ident
        where #(#where_clauses)* {
            fn spawn_world(
                __config_world: &mut #crate_path::__import::World,
                __config_ctx: #crate_path::SpawnContext,
                _: &Self::Metadata,
            ) -> Self::SpawnHandle { #spawn_world }
        }
    }
}

fn gen_spawn_world(crate_path: &syn::Path, idents: &Idents, input: &Input) -> TokenStream {
    let spawn_handle_ident = &idents.spawn_handle_ident;
    let spawn_fields = input.data.iter_field_data().map(|field| {
        let field_ident = &field.spawn_handle_field;
        let field_ty = &field.ty;
        let hierarchy_key = &field.hierarchy_key;
        let metadata_paths = field.metadata.iter().map(|entry| &entry.path);
        let metadata_values = field.metadata.iter().map(|entry| &entry.value);
        let metadata = quote! {{
            type __Struct<T> = T;
            let mut __config_metadata = <__Struct<
                <#field_ty as #crate_path::ConfigField>::Metadata,
            > as #crate_path::__import::Default>::default();
            #(
                __config_metadata.#metadata_paths = #metadata_values;
            )*
            __config_metadata
        }};
        quote! {
            #field_ident: <#field_ty as #crate_path::ConfigFieldFor<__ConfigManager>>::spawn_world(
                __config_world,
                __config_ctx.join(#hierarchy_key),
                &#metadata,
            ),
        }
    });
    quote! {
        #spawn_handle_ident {
            #(#spawn_fields)*
        }
    }
}

fn gen_read_world(crate_path: &syn::Path, idents: &Idents, input: &Input) -> TokenStream {
    match input.data {
        InputData::Struct(ref struct_input) => {
            gen_read_world_struct(crate_path, idents, struct_input)
        }
        InputData::Enum(ref enum_input) => gen_read_world_enum(crate_path, idents, enum_input),
    }
}

fn gen_read_world_struct(
    crate_path: &syn::Path,
    idents: &Idents,
    input: &StructInput,
) -> TokenStream {
    let read_ident = &idents.read_ident;

    let read_fields = input.fields.iter().map(|field| {
        let field_ident = &field.ident;
        let field_ty = &field.data.ty;
        let spawn_handle_ident = &field.data.spawn_handle_field;
        quote! {
            #field_ident: <#field_ty as #crate_path::ConfigField>::read_world(
                __config_query,
                &__config_spawn_handle.#spawn_handle_ident,
            )
        }
    });

    quote! {
        #read_ident {
            #(#read_fields,)*
        }
    }
}

fn gen_read_world_enum(crate_path: &syn::Path, idents: &Idents, input: &EnumInput) -> TokenStream {
    let discrim_spawn_handle_field = &input.discrim.spawn_handle_field;
    let discrim_ident = idents.discrim_ident().expect("Enum must have a discriminant type");
    let discrim = quote! {(
        <#discrim_ident as #crate_path::ConfigField>::read_world(
            __config_query,
            &__config_spawn_handle.#discrim_spawn_handle_field,
        )
    )};

    let read_ident = &idents.read_ident;
    let read_variants = input.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let variant_fields = variant.fields.iter().map(|field| {
            let field_ident = &field.ident;
            let field_ty = &field.data.ty;
            let spawn_handle_ident = &field.data.spawn_handle_field;
            quote! {
                #field_ident: <#field_ty as #crate_path::ConfigField>::read_world(
                    __config_query,
                    &__config_spawn_handle.#spawn_handle_ident,
                ),
            }
        });

        quote! {
            #discrim_ident::#variant_ident => #read_ident::#variant_ident {
                #(#variant_fields)*
            },
        }
    });

    quote! {
        match #discrim {
            #(#read_variants)*
        }
    }
}

fn gen_changed_fn(crate_path: &syn::Path, idents: &Idents, input: &Input) -> TokenStream {
    match input.data {
        InputData::Struct(ref struct_input) => {
            gen_changed_fn_struct(crate_path, idents, struct_input)
        }
        InputData::Enum(ref enum_input) => gen_changed_fn_enum(crate_path, idents, enum_input),
    }
}

fn gen_changed_fn_struct(
    crate_path: &syn::Path,
    idents: &Idents,
    input: &StructInput,
) -> TokenStream {
    let changed_ident = &idents.changed_ident;

    let changed_fields = input.fields.iter().map(|field| {
        let field_ident = &field.ident;
        let field_ty = &field.data.ty;
        let spawn_handle_ident = &field.data.spawn_handle_field;
        quote! {
            #field_ident: <#field_ty as #crate_path::ConfigField>::changed(
                __config_query,
                &__config_spawn_handle.#spawn_handle_ident,
            )
        }
    });

    quote! {
        #changed_ident {
            #(#changed_fields,)*
        }
    }
}

fn gen_changed_fn_enum(crate_path: &syn::Path, idents: &Idents, input: &EnumInput) -> TokenStream {
    let discrim_spawn_handle_field = &input.discrim.spawn_handle_field;
    let discrim_ident = idents.discrim_ident().expect("Enum must have a discriminant type");
    let discrim = quote! {(
        __config_query.get(__config_spawn_handle.#discrim_spawn_handle_field).expect(
            "entity managed by config field must remain active as long as the config handle is used"
        ).1.get::<#crate_path::ScalarData<#discrim_ident>>().expect(
            "discriminant entity must have been spawned with a ScalarData of the corresponding type",
        ).0
    )};

    let changed_ident = &idents.changed_ident;
    let changed_variants = input.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let variant_fields = variant.fields.iter().map(|field| {
            let field_ident = &field.ident;
            let field_ty = &field.data.ty;
            let spawn_handle_ident = &field.data.spawn_handle_field;
            quote! {
                #field_ident: <#field_ty as #crate_path::ConfigField>::changed(
                    __config_query,
                    &__config_spawn_handle.#spawn_handle_ident,
                ),
            }
        });

        quote! {
            #discrim_ident::#variant_ident => #changed_ident::#variant_ident {
                #(#variant_fields)*
            },
        }
    });

    quote! {
        match #discrim {
            #(#changed_variants)*
        }
    }
}

fn dead_code_workaround(input: &Input) -> TokenStream {
    let input_ident = &input.ident;
    let body = match &input.data {
        InputData::Struct(struct_input) => struct_input
            .fields
            .iter()
            .map(|field| {
                let field_ident = &field.ident;
                quote! {
                    drop(v.#field_ident);
                }
            })
            .collect::<TokenStream>(),
        InputData::Enum(enum_input) => {
            let variant_ctors = enum_input.variants.iter().map(|variant| {
                let variant_ident = &variant.ident;
                let ctor_fn_ident = format_ident!("ctor_{variant_ident}");
                let (variant_fields, params): (Vec<_>, Vec<_>) = variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        let field_ident = &field.ident;
                        let binding = syn::Ident::new(&format!("field_{index}"), field.span);
                        let field_ty = &field.data.ty;
                        (quote!(#field_ident: #binding), quote!(#binding: #field_ty))
                    })
                    .unzip();
                quote! {
                    fn #ctor_fn_ident(#(#params),*) -> #input_ident {
                        #input_ident::#variant_ident {
                            #(#variant_fields),*
                        }
                    }
                }
            });

            let variant_users = enum_input.variants.iter().map(|variant| {
                let variant_ident = &variant.ident;
                let (variant_fields, drop_fields): (Vec<_>, Vec<_>) = variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        let field_ident = &field.ident;
                        let binding = syn::Ident::new(&format!("field_{index}"), field.span);
                        (quote!(#field_ident: #binding), quote!(drop(#binding);))
                    })
                    .unzip();
                quote! {
                    #input_ident::#variant_ident { #(#variant_fields),* } => {
                        #(#drop_fields)*
                    }
                }
            });
            quote! {
                #(#variant_ctors)*

                match v {
                    #(#variant_users)*
                }
            }
        }
    };
    quote! {
        #[allow(dead_code)]
        fn dead_code_workaround(v: #input_ident) {
            #body
        }
    }
}

struct ItemAttrs {
    crate_path:          syn::Path,
    debug_print:         bool,
    expose_spawn_handle: bool,
    expose_read:         bool,
    expose_changed:      bool,
    expose_discrim:      bool,
    discrim_metadata:    Vec<MetadataEntry>,
}

impl Default for ItemAttrs {
    fn default() -> Self {
        Self {
            crate_path:          syn::parse_quote!(::bevy_mod_config),
            debug_print:         false,
            expose_spawn_handle: false,
            expose_read:         false,
            expose_changed:      false,
            expose_discrim:      false,
            discrim_metadata:    Vec::new(),
        }
    }
}

struct ItemAttrParse {
    items: Punctuated<ItemAttrParseItem, syn::Token![,]>,
}

impl Parse for ItemAttrParse {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let items = Punctuated::<ItemAttrParseItem, syn::Token![,]>::parse_terminated_with(
            input,
            |input| {
                let lookahead = input.lookahead1();
                if lookahead.peek(kw::crate_path) {
                    input.parse::<kw::crate_path>()?;
                    let inner;
                    syn::parenthesized!(inner in input);
                    let path: syn::Path = inner.parse()?;
                    if !inner.is_empty() {
                        return Err(syn::Error::new(
                            inner.span(),
                            "crate_path can only contain a single path",
                        ));
                    }
                    Ok(ItemAttrParseItem::CratePath(path))
                } else if lookahead.peek(kw::__debug_print) {
                    input.parse::<kw::__debug_print>()?;
                    Ok(ItemAttrParseItem::DebugPrint)
                } else if lookahead.peek(kw::expose) {
                    input.parse::<kw::expose>()?;
                    let exposed = input
                        .peek(syn::token::Paren)
                        .then(|| {
                            let inner;
                            syn::parenthesized!(inner in input);
                            inner.parse_terminated(ItemAttrExposeItem::parse, syn::Token![,])
                        })
                        .transpose()?;
                    Ok(ItemAttrParseItem::Expose(exposed))
                } else if lookahead.peek(kw::discrim) {
                    input.parse::<kw::discrim>()?;
                    let inner;
                    syn::parenthesized!(inner in input);
                    let metadata = inner.parse_terminated(MetadataEntry::parse, syn::Token![,])?;
                    if !inner.is_empty() {
                        return Err(syn::Error::new(
                            inner.span(),
                            "discrim metadata can only contain a single path",
                        ));
                    }
                    Ok(ItemAttrParseItem::DiscrimMetadata(metadata))
                } else {
                    Err(lookahead.error())
                }
            },
        )?;
        Ok(Self { items })
    }
}

enum ItemAttrParseItem {
    CratePath(syn::Path),
    DebugPrint,
    Expose(Option<Punctuated<ItemAttrExposeItem, syn::Token![,]>>),
    DiscrimMetadata(Punctuated<MetadataEntry, syn::Token![,]>),
}

enum ItemAttrExposeItem {
    SpawnHandle,
    Read,
    Changed,
    Discrim,
}

impl Parse for ItemAttrExposeItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::spawn_handle) {
            input.parse::<kw::spawn_handle>()?;
            Ok(ItemAttrExposeItem::SpawnHandle)
        } else if lookahead.peek(kw::read) {
            input.parse::<kw::read>()?;
            Ok(ItemAttrExposeItem::Read)
        } else if lookahead.peek(kw::changed) {
            input.parse::<kw::changed>()?;
            Ok(ItemAttrExposeItem::Changed)
        } else if lookahead.peek(kw::discrim) {
            input.parse::<kw::discrim>()?;
            Ok(ItemAttrExposeItem::Discrim)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ItemAttrParseItem {
    fn apply(self, attrs: &mut ItemAttrs) {
        match self {
            ItemAttrParseItem::CratePath(path) => {
                attrs.crate_path = path;
            }
            ItemAttrParseItem::DebugPrint => {
                attrs.debug_print = true;
            }
            ItemAttrParseItem::Expose(None) => {
                attrs.expose_spawn_handle = true;
                attrs.expose_read = true;
                attrs.expose_changed = true;
                attrs.expose_discrim = true;
            }
            ItemAttrParseItem::Expose(Some(exposed)) => {
                for item in exposed {
                    match item {
                        ItemAttrExposeItem::SpawnHandle => attrs.expose_spawn_handle = true,
                        ItemAttrExposeItem::Read => attrs.expose_read = true,
                        ItemAttrExposeItem::Changed => attrs.expose_changed = true,
                        ItemAttrExposeItem::Discrim => attrs.expose_discrim = true,
                    }
                }
            }
            ItemAttrParseItem::DiscrimMetadata(metadata) => {
                attrs.discrim_metadata.extend(metadata);
            }
        }
    }
}

mod kw {
    syn::custom_keyword!(crate_path);
    syn::custom_keyword!(__debug_print);
    syn::custom_keyword!(expose);
    syn::custom_keyword!(spawn_handle);
    syn::custom_keyword!(read);
    syn::custom_keyword!(changed);
    syn::custom_keyword!(discrim);
}

struct Idents {
    spawn_handle_ident: syn::Ident,
    read_ident:         syn::Ident,
    changed_ident:      syn::Ident,
    discrim_ty:         Option<syn::Type>,
}

impl Idents {
    fn new(input: &syn::DeriveInput) -> syn::Result<Self> {
        let input_ident = &input.ident;
        let spawn_handle_ident = format_ident!("{input_ident}SpawnHandle");
        let read_ident = format_ident!("{input_ident}Read");
        let changed_ident = format_ident!("{input_ident}Changed");
        let discrim_ty = match &input.data {
            syn::Data::Enum(_) => Some(syn::Type::Path(syn::TypePath {
                qself: None,
                path:  format_ident!("{input_ident}Discrim").into(),
            })),
            _ => None,
        };

        Ok(Self { spawn_handle_ident, read_ident, changed_ident, discrim_ty })
    }

    fn discrim_ident(&self) -> Option<&syn::Ident> {
        match self.discrim_ty {
            Some(syn::Type::Path(ref type_path)) => type_path.path.get_ident(),
            _ => None,
        }
    }
}

struct Input<'a> {
    ident: &'a syn::Ident,
    vis:   &'a syn::Visibility,
    data:  InputData<'a>,
}

impl<'a> Input<'a> {
    fn new(
        input: &'a syn::DeriveInput,
        item_attrs: &ItemAttrs,
        idents: &'a Idents,
    ) -> syn::Result<Self> {
        let data = InputData::new(input, item_attrs, idents)?;
        Ok(Self { ident: &input.ident, vis: &input.vis, data })
    }
}

enum InputData<'a> {
    Struct(StructInput<'a>),
    Enum(EnumInput<'a>),
}

impl<'a> InputData<'a> {
    fn new(
        input: &'a syn::DeriveInput,
        item_attrs: &ItemAttrs,
        idents: &'a Idents,
    ) -> syn::Result<Self> {
        match &input.data {
            syn::Data::Struct(data_struct) => Ok(InputData::Struct(StructInput::new(data_struct)?)),

            syn::Data::Enum(data_enum) => {
                Ok(InputData::Enum(EnumInput::new(data_enum, item_attrs, idents)?))
            }

            _ => Err(syn::Error::new_spanned(
                input,
                "Config can only be derived for structs and enums",
            )),
        }
    }

    fn iter_field_data(&self) -> impl Iterator<Item = &InputFieldData<'a>> {
        match self {
            InputData::Struct(struct_input) => {
                Either::Left(struct_input.fields.iter().map(|field| &field.data))
            }
            InputData::Enum(enum_input) => Either::Right(
                iter::once(&enum_input.discrim).chain(
                    enum_input
                        .variants
                        .iter()
                        .flat_map(|variant| variant.fields.iter().map(|field| &field.data)),
                ),
            ),
        }
    }
}

struct StructInput<'a> {
    named_fields: bool,
    fields:       Vec<InputField<'a>>,
}

impl<'a> StructInput<'a> {
    fn new(data: &'a syn::DataStruct) -> syn::Result<Self> {
        let fields = data
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let (ident, spawn_handle_field) = match field.ident {
                    None => (
                        InputFieldIdent::Index(index),
                        syn::Ident::new(&format!("field_{index}"), field.span()),
                    ),
                    Some(ref ident) => {
                        (InputFieldIdent::Ident(ident), format_ident!("field_{ident}"))
                    }
                };
                let hierarchy_key = match ident {
                    InputFieldIdent::Index(index) => index.to_string(),
                    InputFieldIdent::Ident(ident) => ident.to_string(),
                };
                let metadata = metadata_from_attrs(&field.attrs)?;
                Ok(InputField {
                    vis: &field.vis,
                    ident,
                    span: field.span(),
                    data: InputFieldData {
                        ty: &field.ty,
                        spawn_handle_field,
                        hierarchy_key,
                        metadata,
                    },
                })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(Self { fields, named_fields: matches!(data.fields, syn::Fields::Named(_)) })
    }
}

struct EnumInput<'a> {
    discrim:  InputFieldData<'a>,
    variants: Vec<EnumVariant<'a>>,
}

impl<'a> EnumInput<'a> {
    fn new(
        data: &'a syn::DataEnum,
        item_attrs: &ItemAttrs,
        idents: &'a Idents,
    ) -> syn::Result<Self> {
        let discrim = InputFieldData {
            ty:                 idents.discrim_ty.as_ref().unwrap(),
            spawn_handle_field: format_ident!("discrim"),
            hierarchy_key:      "discrim".to_string(),
            metadata:           item_attrs.discrim_metadata.clone(),
        };

        let variants = data
            .variants
            .iter()
            .map(|variant| {
                let fields = variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        let (ident, spawn_handle_field) = match field.ident {
                            None => (
                                InputFieldIdent::Index(index),
                                format_ident!("variant_{}_field_{index}", variant.ident),
                            ),
                            Some(ref ident) => (
                                InputFieldIdent::Ident(ident),
                                format_ident!("variant_{}_field_{ident}", &variant.ident),
                            ),
                        };
                        let hierarchy_key = match ident {
                            InputFieldIdent::Index(index) => format!("{}:{}", variant.ident, index),
                            InputFieldIdent::Ident(ident) => format!("{}:{}", variant.ident, ident),
                        };
                        let metadata = metadata_from_attrs(&field.attrs)?;
                        Ok(InputField {
                            vis: &field.vis,
                            ident,
                            span: field.span(),
                            data: InputFieldData {
                                ty: &field.ty,
                                spawn_handle_field,
                                hierarchy_key,
                                metadata,
                            },
                        })
                    })
                    .collect::<syn::Result<Vec<_>>>()?;

                Ok(EnumVariant {
                    ident: &variant.ident,
                    field_syntax: match variant.fields {
                        syn::Fields::Named(_) => FieldSyntax::Named,
                        syn::Fields::Unnamed(_) => FieldSyntax::Unnamed,
                        syn::Fields::Unit => FieldSyntax::Unit,
                    },
                    fields,
                })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        if variants.is_empty() {
            return Err(syn::Error::new_spanned(
                &data.variants,
                "Config enums must have at least one variant",
            ));
        }

        Ok(Self { discrim, variants })
    }
}

type MetadataPath = Punctuated<syn::Ident, syn::Token![.]>;

#[derive(Clone)]
struct MetadataEntry {
    path:  MetadataPath,
    value: syn::Expr,
}

impl Parse for MetadataEntry {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path = Punctuated::<syn::Ident, syn::Token![.]>::parse_separated_nonempty(input)?;
        let _: syn::Token![=] = input.parse()?;
        let value: syn::Expr = input.parse()?;
        Ok(Self { path, value })
    }
}

fn metadata_from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Vec<MetadataEntry>> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("config"))
        .flat_map(|attr| match parse_config_metadata(attr) {
            Ok(metadata) => Either::Left(metadata.into_iter().map(Ok)),
            Err(err) => Either::Right(iter::once(Err(err))),
        })
        .collect()
}

fn parse_config_metadata(attr: &syn::Attribute) -> syn::Result<Vec<MetadataEntry>> {
    let punctuated =
        attr.parse_args_with(Punctuated::<MetadataEntry, syn::Token![,]>::parse_terminated)?;
    Ok(punctuated.into_iter().collect())
}

struct EnumVariant<'a> {
    ident:        &'a syn::Ident,
    field_syntax: FieldSyntax,
    fields:       Vec<InputField<'a>>,
}

enum FieldSyntax {
    Named,
    Unnamed,
    Unit,
}

struct InputField<'a> {
    vis:   &'a syn::Visibility,
    ident: InputFieldIdent<'a>,
    span:  Span,
    data:  InputFieldData<'a>,
}

enum InputFieldIdent<'a> {
    Index(usize),
    Ident(&'a syn::Ident),
}

impl<'a> InputFieldIdent<'a> {
    fn ident(&self) -> Option<&'a syn::Ident> {
        match *self {
            InputFieldIdent::Index(_) => None,
            InputFieldIdent::Ident(ident) => Some(ident),
        }
    }
}

impl ToTokens for InputFieldIdent<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            InputFieldIdent::Index(index) => {
                syn::LitInt::new(&format!("{index}"), Span::call_site()).to_tokens(tokens)
            }
            InputFieldIdent::Ident(ref ident) => tokens.extend(quote!(#ident)),
        }
    }
}

struct InputFieldData<'a> {
    ty:                 &'a syn::Type,
    spawn_handle_field: syn::Ident,
    hierarchy_key:      String,
    metadata:           Vec<MetadataEntry>,
}
