use proc_macro::TokenStream as InterfaceTokenStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, Stmt, parse};

#[proc_macro_attribute]
pub fn debug_condition(condition: InterfaceTokenStream, conditional: InterfaceTokenStream) -> InterfaceTokenStream {
    let condition = TokenStream::from(condition);
    let conditional = TokenStream::from(conditional);

    quote! {
        if #condition {
            #conditional
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn profile(name: InterfaceTokenStream, function: InterfaceTokenStream) -> InterfaceTokenStream {
    let mut function: ItemFn = parse(function).expect("failed to parse token stream");
    let name: LitStr = parse(name).unwrap_or_else(|_| {
        let function_name = &function.sig.ident;
        LitStr::new(function_name.to_string().replace('_', " ").as_str(), function_name.span())
    });

    let code = quote! {
        let _measurement = korangar_debug::profiling::Profiler::start_measurement(#name);
    }
    .into();

    let statement: Stmt = parse(code).expect("failed to parse token stream");
    function.block.stmts.insert(0, statement);

    quote! { #function }.into()
}
