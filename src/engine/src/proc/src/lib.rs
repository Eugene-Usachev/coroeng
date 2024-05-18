#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn coro(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_block = &input.block;
    let fn_args = &input.sig.inputs;
    let fn_vis = &input.vis;
    let fn_generics = &input.sig.generics;

    let expanded = quote! {
        #[inline(always)]
        #fn_vis fn #fn_name<#fn_generics>(#fn_args) -> engine::coroutine::CoroutineImpl {
            std::boxed::Box::pin(#[coroutine] static move || {
                #fn_block
            })
        }
    };

    TokenStream::from(expanded)
}