#![feature(extract_if)]

use case::CaseExt;
use proc_macro::TokenStream as InterfaceTokenStream;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_quote, DeriveInput};

#[proc_macro_derive(RustState, attributes(state_root))]
pub fn derive_prototype_element(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        attrs,
        data,
        generics,
        ..
    } = syn::parse(token_stream).expect("failed to parse token stream");

    let is_root = attrs
        .iter()
        .filter_map(|attribute| syn::parse::<syn::Ident>(attribute.meta.to_token_stream().into()).ok())
        .any(|ident| ident.to_string().as_str() == "state_root");

    match is_root {
        true => impl_for_root(ident, data, generics),
        false => impl_for_inner(ident, data, generics),
    }
    .into()
}

fn impl_for_root(ident: syn::Ident, data: syn::Data, generics: syn::Generics) -> TokenStream {
    let mut selector_generics = generics.clone();
    selector_generics.params.push(parse_quote!('_a));
    let selector_impl_generics = selector_generics.split_for_impl().0;

    let lifetimes = generics.lifetimes().map(|lifetime| quote!(&#lifetime ())).collect::<Vec<_>>();
    let type_params = generics.type_params().map(|type_param| quote!(#type_param)).collect::<Vec<_>>();

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let mut base_getters = Vec::new();

    match data {
        syn::Data::Struct(data_struct) => {
            for (index, field) in data_struct.fields.into_iter().enumerate() {
                let field_name = field.ident.as_ref().unwrap();
                let struct_name = syn::Ident::new(
                    &format!("{}{}Path", ident, field.ident.as_ref().unwrap().to_string().to_camel()),
                    field.ident.as_ref().unwrap().span(),
                );
                let getter_name = syn::Ident::new(
                    &format!("{}", field.ident.as_ref().unwrap()),
                    field.ident.as_ref().unwrap().span(),
                );
                let field_type = field.ty;
                let uuid = index as u32;

                base_getters.push(quote! {
                    #[derive(Clone)]
                    pub struct #struct_name #type_generics #where_clause {
                        _marker: std::marker::PhantomData<(#(#lifetimes,)* #(#type_params,)*)>,
                    }

                    impl #selector_impl_generics rust_state::Selector<'_a, #ident #type_generics, #field_type> for #struct_name #type_generics #where_clause {
                        fn get(&self, state: &'_a #ident #type_generics) -> Option<&'_a #field_type> {
                            Some(&state.#field_name)
                        }

                        fn get_mut(&self, state: &'_a mut #ident #type_generics) -> Option<&'_a mut #field_type> {
                            Some(&mut state.#field_name)
                        }

                        fn get_path_id(&self) -> rust_state::PathId {
                            rust_state::PathId::new(vec![rust_state::PathUuid(#uuid)])
                        }
                    }

                    impl #impl_generics #ident #type_generics #where_clause {
                        pub fn #getter_name() -> #struct_name #type_generics {
                            #struct_name {
                                _marker: std::marker::PhantomData,
                            }
                        }
                    }
                });
            }
        }
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => panic!("foob"),
    }

    quote! {
        impl #impl_generics rust_state::StateMarker for #ident #type_generics #where_clause {}
        #(#base_getters)*
    }
}

fn impl_for_inner(ident: syn::Ident, data: syn::Data, generics: syn::Generics) -> TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let lifetimes = generics.lifetimes().map(|lifetime| quote!(&#lifetime ())).collect::<Vec<_>>();
    let type_params = generics.type_params().map(|type_param| quote!(#type_param)).collect::<Vec<_>>();

    let mut struct_generics = generics.clone();
    struct_generics.params.push(parse_quote!(S));
    struct_generics.params.push(parse_quote!(P));
    let (struct_impl_generics, struct_type_generics, struct_where_clause) = struct_generics.split_for_impl();

    let mut clone_generics = struct_generics.clone();
    let clone_where_clause = clone_generics.make_where_clause();
    clone_where_clause.predicates.push(parse_quote!(P: Clone));

    let mut selector_generics = generics.clone();
    selector_generics.params.push(parse_quote!('_a));
    selector_generics.params.push(parse_quote!(S: rust_state::StateMarker));
    selector_generics
        .params
        .push(parse_quote!(P: rust_state::Selector<'_a, S, #ident #type_generics> + Clone));
    let (selector_impl_generics, _, selector_where_clause) = selector_generics.split_for_impl();

    let mut base_getters = Vec::new();

    match data {
        syn::Data::Struct(data_struct) => {
            for (index, field) in data_struct.fields.into_iter().enumerate() {
                let field_name = field.ident.as_ref().unwrap();
                let struct_name = syn::Ident::new(
                    &format!("{}{}Path", ident, field.ident.as_ref().unwrap().to_string().to_camel()),
                    field.ident.as_ref().unwrap().span(),
                );
                let getter_name = syn::Ident::new(
                    &format!("{}", field.ident.as_ref().unwrap()),
                    field.ident.as_ref().unwrap().span(),
                );
                let field_type = field.ty;
                let uuid = index as u32;

                base_getters.push(quote! {
                    pub struct #struct_name #struct_type_generics #struct_where_clause {
                        path: P,
                        _marker: std::marker::PhantomData<(S, #(#lifetimes,)* #(#type_params,)*)>,
                    }

                    impl #struct_impl_generics Clone for #struct_name #struct_type_generics #clone_where_clause
                    {
                        fn clone(&self) -> Self {
                            Self {
                                path: self.path.clone(),
                                _marker: std::marker::PhantomData,
                            }
                        }
                    }

                    impl #selector_impl_generics rust_state::Selector<'_a, S, #field_type> for #struct_name #struct_type_generics #selector_where_clause {
                        fn get(&self, state: &'_a S) -> Option<&'_a #field_type> {
                            Some(&self.path.get(state)?.#field_name)
                        }

                        fn get_mut(&self, state: &'_a mut S) -> Option<&'_a mut #field_type> {
                            Some(&mut self.path.get_mut(state)?.#field_name)
                        }

                        fn get_path_id(&self) -> rust_state::PathId {
                            let mut inner = self.path.get_path_id();
                            inner.push(rust_state::PathUuid(#uuid));
                            inner
                        }
                    }

                    impl #impl_generics #ident #type_generics #where_clause {
                        pub fn #getter_name<S, P>(path: P) -> #struct_name #struct_type_generics {
                            #struct_name { path, _marker: std::marker::PhantomData }
                        }
                    }
                });
            }
        }
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => panic!("foob"),
    }

    quote! {
        #(#base_getters)*
    }
}
