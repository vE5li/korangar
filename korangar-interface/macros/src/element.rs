use proc_macro::TokenStream as InterfaceTokenStream;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, DataEnum, DataStruct, Generics, Ident};

use super::helper::{field_display_name, state_element_helper};
use super::utils::get_unique_attribute;

pub fn derive_state_element_struct(
    data_struct: DataStruct,
    generics: Generics,
    attributes: Vec<Attribute>,
    name: Ident,
) -> InterfaceTokenStream {
    let (initializers, initializers_mut, is_unnamed, _window_title, _window_class) =
        state_element_helper(data_struct, attributes, name.to_string());
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    // TODO: Instead get this from the proc macro.
    let impl_for = match std::env::var("CARGO_PKG_NAME").unwrap() == "korangar" {
        true => Some(quote!(crate::state::ClientState)),
        false => None,
    };

    if let Some(impl_for) = impl_for {
        if initializers.len() == 1 && is_unnamed {
            return quote! {
                impl #impl_generics korangar_interface::element::StateElement<#impl_for> for #name #type_generics #where_clause {
                    type LayoutInfoMut = impl std::any::Any;
                    type ReturnMut<P>
                        = impl korangar_interface::element::Element<#impl_for, LayoutInfo = Self::LayoutInfoMut>
                    where
                        P: rust_state::Path<#impl_for, Self>;
                    type LayoutInfo = impl std::any::Any;
                    type Return<P>
                        = impl korangar_interface::element::Element<#impl_for, LayoutInfo = Self::LayoutInfo>
                    where
                        P: rust_state::Path<#impl_for, Self>;

                    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
                        where P: rust_state::Path<#impl_for, Self>
                    {
                        korangar_interface::element::StateElement::to_element(self_path._0(), name)
                    }

                    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
                        where P: rust_state::Path<#impl_for, Self>
                    {
                        korangar_interface::element::StateElement::to_element_mut(self_path._0(), name)
                    }
                }
            }
            .into();
        }

        return quote! {
            impl #impl_generics korangar_interface::element::StateElement<#impl_for> for #name #type_generics #where_clause {
                type LayoutInfoMut = impl std::any::Any;
                type ReturnMut<P>
                    = impl korangar_interface::element::Element<#impl_for, LayoutInfo = Self::LayoutInfoMut>
                where
                    P: rust_state::Path<#impl_for, Self>;
                type LayoutInfo = impl std::any::Any;
                type Return<P>
                    = impl korangar_interface::element::Element<#impl_for, LayoutInfo = Self::LayoutInfo>
                where
                    P: rust_state::Path<#impl_for, Self>;

                fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
                    where P: rust_state::Path<#impl_for, Self>
                {
                    use korangar_interface::prelude::*;

                    collapsible! {
                        text: name,
                        children: (#(#initializers,)*),
                    }
                }

                fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
                    where P: rust_state::Path<#impl_for, Self>
                {
                    use korangar_interface::prelude::*;

                    collapsible! {
                        text: name,
                        children: (#(#initializers_mut,)*),
                    }
                }
            }
        }
        .into();
    }

    if initializers.len() == 1 && is_unnamed {
        return quote! {
            impl<App: korangar_interface::application::Application> #impl_generics korangar_interface::element::StateElement<App> for #name #type_generics #where_clause {
                type LayoutInfoMut = impl std::any::Any;
                type ReturnMut<P>
                    = impl korangar_interface::element::Element<App, LayoutInfo = Self::LayoutInfoMut>
                where
                    P: rust_state::Path<App, Self>;
                type LayoutInfo = impl std::any::Any;
                type Return<P>
                    = impl korangar_interface::element::Element<App, LayoutInfo = Self::LayoutInfo>
                where
                    P: rust_state::Path<App, Self>;

                fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
                    where P: rust_state::Path<App, Self>
                {
                    korangar_interface::element::StateElement::to_element(self_path._0(), name)
                }

                fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
                    where P: rust_state::Path<App, Self>
                {
                    korangar_interface::element::StateElement::to_element_mut(self_path._0(), name)
                }
            }
        }
        .into();
    }

    quote! {
        impl<App: korangar_interface::application::Application> #impl_generics korangar_interface::element::StateElement<App> for #name #type_generics #where_clause {
            type LayoutInfoMut = impl std::any::Any;
            type ReturnMut<P>
                = impl korangar_interface::element::Element<App, LayoutInfo = Self::LayoutInfoMut>
            where
                P: rust_state::Path<App, Self>;
            type LayoutInfo = impl std::any::Any;
            type Return<P>
                = impl korangar_interface::element::Element<App, LayoutInfo = Self::LayoutInfo>
            where
                P: rust_state::Path<App, Self>;

            fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
                where P: rust_state::Path<App, Self>
            {
                use korangar_interface::prelude::*;

                collapsible! {
                    text: name,
                    children: (#(#initializers,)*),
                }
            }

            fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
                where P: rust_state::Path<App, Self>
            {
                use korangar_interface::prelude::*;

                collapsible! {
                    text: name,
                    children: (#(#initializers_mut,)*),
                }
            }
        }
    }
    .into()
}

/// Derive [`StateElement`] for an enum.
///
/// - Variants without (visible) fields are displayed as `name` next to the name
///   of the active variant.
/// - Variants with visible fields are displayed as a collapsible titled `name:
///   Variant`, containing the elements of all fields. Fields can be hidden with
///   `#[hidden_element]` and renamed with `#[name("...")]`, just like struct
///   fields. Every visible field type needs to implement [`StateElement`].
///
/// Generic enums are not supported.
///
/// # Compile time considerations
///
/// Some enums have a *lot* of variants (e.g. `EffectId` has more than a
/// thousand), so the generated code is structured to keep type checking cheap.
/// A previous version built one `split! { text!, field! }` per variant inline
/// in `to_element` and stored them in a struct with one generic parameter per
/// variant. That version took hours to type check `EffectId`, and it got
/// drastically slower on newer nightlies. There were two reasons:
///
/// 1. The component macros call `korangar_interface::theme::theme()` without a
///    turbofish, so every theme selector introduces a fresh inference variable
///    for `App`. None of the component types mention `App`, so those variables
///    are only unified with the real `App` when the final return value is
///    checked. Until then, all of their obligations stay pending, and rustc
///    re-processes the pending set at every method call. The cost is quadratic
///    in the size of the function body. TAIT is *not* the problem: a plain `->
///    impl Element<App>` function behaves the same.
/// 2. Even with `App` resolved early, a struct with over a thousand generic
///    parameters (and matching where clauses) is very expensive on its own.
///
/// The generated code avoids both:
///
/// - All variants without visible fields share a *single* element that uses a
///   selector to display the name of the active variant. The cost of those
///   variants therefore doesn't depend on how many there are.
/// - Every element is passed through `__pin::<App, _>(...)` right where it is
///   built. This resolves the `App` inference variables immediately, so the set
///   of pending obligations stays small. **Never build many elements in one
///   function body without pinning `App`.**
///
/// Note that the elements can't be built in separate helper functions
/// returning `impl Element<App>`, even though that would isolate their type
/// checking even better. [`StateElement::LayoutInfo`] is not generic over the
/// path type `P`, but the layout info of an opaque type returned from a
/// function generic over `P` is considered to depend on `P` (and precise
/// capturing can't exclude type parameters yet). Building the elements inline
/// keeps their concrete (P-independent) layout info visible.
///
/// # Restriction
///
/// Each variant *with* visible fields still adds its element inline, one
/// generic parameter to `Inner`, and one variant to `InnerLayoutInfo`. This is
/// fine for the handful of data variants that exist right now, but an enum with
/// hundreds of data variants would run into the problems described above
/// again (large function body, many generic parameters).
///
/// Possible solutions if that ever becomes necessary:
///
/// - Wrap each data variant element in
///   [`ErasedElement::new`](korangar_interface::element::ErasedElement::new)
///   and store it as an [`ElementBox`](korangar_interface::element::ElementBox)
///   (e.g. in a `Vec` or array indexed by the variant). The layout info of an
///   erased element is `()`, so this also works from helper functions generic
///   over `P`, and `Inner` has a fixed number of generic parameters. The cost
///   is dynamic dispatch and an allocation per variant.
/// - Group data variants that produce the same element type (e.g. same field
///   types in the same order) and store each group in an array, so the number
///   of generic parameters depends on the number of distinct shapes rather than
///   the number of variants.
pub fn derive_state_element_enum(data_enum: DataEnum, generics: Generics, name: Ident) -> InterfaceTokenStream {
    if !generics.params.is_empty() {
        panic!("deriving StateElement is not supported for generic enums");
    }

    let variants = collect_enum_variants(data_enum, &name);

    let to_element_body = enum_element_body(&name, &variants, false);
    let to_element_mut_body = enum_element_body(&name, &variants, true);

    quote! {
        impl<App: korangar_interface::application::Application> korangar_interface::element::StateElement<App> for #name {
            type LayoutInfoMut = impl std::any::Any;
            type ReturnMut<P>
                = impl korangar_interface::element::Element<App, LayoutInfo = Self::LayoutInfoMut>
            where
                P: rust_state::Path<App, Self>;
            type LayoutInfo = impl std::any::Any;
            type Return<P>
                = impl korangar_interface::element::Element<App, LayoutInfo = Self::LayoutInfo>
            where
                P: rust_state::Path<App, Self>;

            fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
                where P: rust_state::Path<App, Self>
            {
                #to_element_body
            }

            fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
                where P: rust_state::Path<App, Self>
            {
                #to_element_mut_body
            }
        }
    }
    .into()
}

/// A visible field of an enum variant.
struct EnumVariantField {
    /// Type of the field.
    ty: syn::Type,
    /// Name displayed in the interface.
    display_name: String,
    /// Pattern that matches the variant and binds the field to `value`.
    pattern: TokenStream,
    /// Name of the generated path type that follows the enum to this field.
    path_type: Ident,
}

struct EnumVariant {
    ident: Ident,
    /// Pattern that matches the variant, ignoring all fields.
    matcher: TokenStream,
    /// Visible fields. If empty, the variant is displayed like a unit variant.
    fields: Vec<EnumVariantField>,
}

fn collect_enum_variants(data_enum: DataEnum, name: &Ident) -> Vec<EnumVariant> {
    data_enum
        .variants
        .into_iter()
        .map(|variant| {
            let variant_ident = variant.ident;

            let matcher = match variant.fields {
                syn::Fields::Named(..) => quote!({ .. }),
                syn::Fields::Unnamed(..) => quote!((..)),
                syn::Fields::Unit => quote!(),
            };

            let is_named = matches!(variant.fields, syn::Fields::Named(..));

            let fields = variant
                .fields
                .into_iter()
                .enumerate()
                .filter_map(|(index, mut field)| {
                    if get_unique_attribute(&mut field.attrs, "hidden_element").is_some() {
                        return None;
                    }

                    let (display_name, pattern) = match is_named {
                        true => {
                            let field_ident = field.ident.clone().unwrap();
                            let display_name = field_display_name(&mut field, &field_ident.to_string());
                            (display_name, quote!(#name::#variant_ident { #field_ident: value, .. }))
                        }
                        false => {
                            let display_name = field_display_name(&mut field, &index.to_string());
                            let skipped = (0..index).map(|_| quote!(_));
                            (display_name, quote!(#name::#variant_ident(#(#skipped,)* value, ..)))
                        }
                    };

                    let path_type = Ident::new(&format!("__Path{variant_ident}{index}"), Span::mixed_site());

                    Some(EnumVariantField {
                        ty: field.ty,
                        display_name,
                        pattern,
                        path_type,
                    })
                })
                .collect();

            EnumVariant {
                ident: variant_ident,
                matcher,
                fields,
            }
        })
        .collect()
}

/// Generate the body of `to_element` or `to_element_mut`.
///
/// See [`derive_state_element_enum`] for why the code is structured the way it
/// is.
fn enum_element_body(name: &Ident, variants: &[EnumVariant], mutable: bool) -> TokenStream {
    let to_element = match mutable {
        true => quote!(to_element_mut),
        false => quote!(to_element),
    };

    // Selector that returns the name of the active variant.
    let all_variant_idents = variants.iter().map(|variant| &variant.ident).collect::<Vec<_>>();
    let all_variant_matchers = variants.iter().map(|variant| &variant.matcher).collect::<Vec<_>>();
    let all_variant_strings = variants.iter().map(|variant| variant.ident.to_string()).collect::<Vec<_>>();

    let data_variants = variants.iter().filter(|variant| !variant.fields.is_empty()).collect::<Vec<_>>();

    let data_variant_idents = data_variants.iter().map(|variant| &variant.ident).collect::<Vec<_>>();
    let data_variant_matchers = data_variants.iter().map(|variant| &variant.matcher).collect::<Vec<_>>();
    let data_generics = data_variants
        .iter()
        .map(|variant| Ident::new(&format!("__{}", variant.ident), Span::mixed_site()))
        .collect::<Vec<_>>();
    let data_element_fields = data_variants
        .iter()
        .map(|variant| Ident::new(&format!("{}_element", variant.ident), Span::mixed_site()))
        .collect::<Vec<_>>();
    // Store index 0 is used by the simple element.
    let data_store_indices = (1..=data_variants.len() as u64).collect::<Vec<_>>();

    // Paths from the enum to the fields of the data variants.
    let field_paths = data_variants.iter().flat_map(|variant| &variant.fields).map(|field| {
        let EnumVariantField {
            ty, pattern, path_type, ..
        } = field;

        quote! {
            struct #path_type<P> {
                path: P,
            }

            impl<P: Copy> Clone for #path_type<P> {
                fn clone(&self) -> Self {
                    *self
                }
            }

            impl<P: Copy> Copy for #path_type<P> {}

            impl<App, P> rust_state::Selector<App, #ty, false> for #path_type<P>
            where
                App: korangar_interface::application::Application,
                P: rust_state::Path<App, #name>,
            {
                fn select<'a>(&'a self, state: &'a App) -> Option<&'a #ty> {
                    <Self as rust_state::Path<App, #ty, false>>::follow(self, state)
                }
            }

            impl<App, P> rust_state::Path<App, #ty, false> for #path_type<P>
            where
                App: korangar_interface::application::Application,
                P: rust_state::Path<App, #name>,
            {
                #[allow(unreachable_patterns)]
                fn follow<'a>(&self, state: &'a App) -> Option<&'a #ty> {
                    match self.path.follow(state)? {
                        #pattern => Some(value),
                        _ => None,
                    }
                }

                #[allow(unreachable_patterns)]
                fn follow_mut<'a>(&self, state: &'a mut App) -> Option<&'a mut #ty> {
                    match self.path.follow_mut(state)? {
                        #pattern => Some(value),
                        _ => None,
                    }
                }
            }
        }
    });

    // One element per data variant.
    let data_elements = data_variants.iter().map(|variant| {
        let variant_string = variant.ident.to_string();
        let field_types = variant.fields.iter().map(|field| &field.ty);
        let field_display_names = variant.fields.iter().map(|field| &field.display_name);
        let field_path_types = variant.fields.iter().map(|field| &field.path_type);

        quote! {
            __pin::<App, _>(collapsible! {
                text: format!("{name}: {}", #variant_string),
                children: (
                    #(
                        // The field path is only valid while this variant is
                        // active. That's fine since `Inner` only creates the
                        // layout for the element of the active variant.
                        <#field_types as korangar_interface::element::StateElement<App>>::#to_element(
                            rust_state::ManuallyAssertExt::<App, #field_types>::manually_asserted(#field_path_types { path: self_path }),
                            #field_display_names.to_string(),
                        ),
                    )*
                ),
            })
        }
    });

    quote! {
        use korangar_interface::prelude::*;

        struct VariantName<P>(P);

        impl<App, P> rust_state::Selector<App, &'static str> for VariantName<P>
        where
            App: korangar_interface::application::Application,
            P: rust_state::Path<App, #name>,
        {
            #[allow(unreachable_code)]
            fn select<'a>(&'a self, state: &'a App) -> Option<&'a &'static str> {
                Some(match *rust_state::PathExt::follow_safe(&self.0, state) {
                    #( #name::#all_variant_idents #all_variant_matchers => &#all_variant_strings, )*
                })
            }
        }

        #(#field_paths)*

        /// Used to resolve `App` for each element right away. See
        /// `derive_state_element_enum` for why this is important.
        #[inline(always)]
        fn __pin<App, E>(element: E) -> E
        where
            App: korangar_interface::application::Application,
            E: korangar_interface::element::Element<App>,
        {
            element
        }

        enum InnerLayoutInfo<Simple, #(#data_generics),*> {
            Simple(Simple),
            #( #data_variant_idents(#data_generics), )*
        }

        struct Inner<P, Simple, #(#data_generics),*> {
            path: P,
            /// Element for all variants without visible fields.
            simple_element: Simple,
            #(
                #[allow(non_snake_case)]
                #data_element_fields: #data_generics,
            )*
        }

        impl<App, P, Simple, #(#data_generics),*> korangar_interface::element::Element<App> for Inner<P, Simple, #(#data_generics),*>
        where
            App: korangar_interface::application::Application,
            P: rust_state::Path<App, #name>,
            Simple: korangar_interface::element::Element<App>,
            #(#data_generics: korangar_interface::element::Element<App>,)*
        {
            type LayoutInfo = InnerLayoutInfo<Simple::LayoutInfo, #(#data_generics::LayoutInfo),*>;

            #[allow(unreachable_patterns)]
            fn create_layout_info(
                &mut self,
                state: &rust_state::State<App>,
                mut store: korangar_interface::element::store::ElementStoreMut,
                resolvers: &mut dyn korangar_interface::layout::Resolvers<App>,
            ) -> Self::LayoutInfo {
                // Every element gets its own child store so element state is not
                // shared between variants.
                match *state.get(&self.path) {
                    #(
                        #name::#data_variant_idents #data_variant_matchers => InnerLayoutInfo::#data_variant_idents(
                            self.#data_element_fields.create_layout_info(state, store.child_store(#data_store_indices), resolvers),
                        ),
                    )*
                    _ => InnerLayoutInfo::Simple(self.simple_element.create_layout_info(state, store.child_store(0), resolvers)),
                }
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a rust_state::State<App>,
                store: korangar_interface::element::store::ElementStore<'a>,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut korangar_interface::layout::WindowLayout<'a, App>,
            ) {
                match layout_info {
                    InnerLayoutInfo::Simple(layout_info) => self.simple_element.lay_out(state, store.child_store(0), layout_info, layout),
                    #(
                        InnerLayoutInfo::#data_variant_idents(layout_info) => {
                            self.#data_element_fields.lay_out(state, store.child_store(#data_store_indices), layout_info, layout)
                        }
                    )*
                }
            }
        }

        // Build the data variant elements first, so `name` can be moved into the
        // simple element.
        #(
            #[allow(non_snake_case)]
            let #data_element_fields = #data_elements;
        )*

        let simple_element = __pin::<App, _>(split! {
            children: (
                text! {
                    text: name,
                },
                field! {
                    text: VariantName(self_path),
                },
            ),
        });

        Inner {
            path: self_path,
            simple_element,
            #( #data_element_fields, )*
        }
    }
}
