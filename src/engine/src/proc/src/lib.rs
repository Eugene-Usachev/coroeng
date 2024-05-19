#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ReturnType, Expr, Stmt};
use syn::visit_mut::{self, VisitMut};

#[proc_macro_attribute]
pub fn print_ret(_: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input_fn = parse_macro_input!(input as ItemFn);

    // Get the identifier of the function
    let fn_name = &input_fn.sig.ident;

    // Get the statements inside the function body
    let fn_body = &input_fn.block.stmts;

    // Generate new statements with println! before each return statement
    let mut new_fn_body = Vec::new();
    for stmt in fn_body {
        match stmt {
            Stmt::Expr(expr, _) => {
                if let Expr::Return(expr) = expr {
                    new_fn_body.push(quote! {
                        println!("{}", #expr);
                        #stmt
                    });
                } else {
                    new_fn_body.push(quote! {
                        #stmt
                    });
                }
            }
            _ => new_fn_body.push(quote! {
                #stmt
            }),
        }
    }

    // Reconstruct the function with modified body
    let output = quote! {
        fn #fn_name() {
            #(#new_fn_body)*
        }
    };

    // Return the generated code as a TokenStream
    output.into()
}

/// This macro is used to convert function into a coroutine creator.
///
/// # Example
///
/// ```rust
/// use engine::net::tcp::{TcpStream, TcpListener};
/// use engine::utils::{io_yield, run_on_all_cores, spawn_local, coro};
///
/// #[coro]
/// fn handle_tcp_client(mut stream: TcpStream) {
///     loop {
///         let slice = io_yield!(TcpStream::read, &mut stream).unwrap();
///
///         if slice.is_empty() {
///             break;
///         }
///
///         let mut buf = engine::utils::buffer();
///         buf.append(slice);
///
///         let res = io_yield!(TcpStream::write_all, &mut stream, buf);
///
///         if res.is_err() {
///             println!("write failed, reason: {}", res.err().unwrap());
///             break;
///         }
///     }
/// }
///
/// fn main() {
///     run_on_all_cores!({
///         let mut listener = io_yield!(TcpListener::new, "localhost:8081".to_socket_addrs().unwrap().next().unwrap());
///         loop {
///             let stream_ = io_yield!(TcpListener::accept, &mut listener);
///
///             if stream_.is_err() {
///                 println!("accept failed, reason: {}", stream_.err().unwrap());
///                 continue;
///             }
///
///             let stream: TcpStream = stream_.unwrap();
///             spawn_local!(handle_tcp_client(stream));
///         }
///     });
/// }
/// ```
///
#[proc_macro_attribute]
pub fn coro(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_args = &input.sig.inputs;
    let fn_vis = &input.vis;
    let fn_generics = &input.sig.generics;
    let fn_return_type = &input.sig.output;

    let has_return_type = match fn_return_type {
        ReturnType::Default => false,
        ReturnType::Type(_, _) => true,
    };

    let expanded;
    if !has_return_type {
        let fn_block = &input.block;
        expanded = quote! {
            #[inline(always)]
            #fn_vis fn #fn_name<#fn_generics>(#fn_args) -> engine::coroutine::CoroutineImpl {
                std::boxed::Box::pin(#[coroutine] static move || {
                    #fn_block
                })
            }
        };
    } else {
        if has_return_type {
            // Create a mutable visitor to transform return statements
            struct ReturnReplacer;

            impl VisitMut for ReturnReplacer {
                fn visit_expr_mut(&mut self, node: &mut Expr) {
                    if let Expr::Return(ret_expr) = node {
                        let new_expr = match &ret_expr.expr {
                            Some(expr) => quote! {
                                *res = #expr;
                                return;

                            },
                            None => quote! {
                                *res = ();
                                return;
                            },
                        };

                        *node = syn::parse2(new_expr).expect("12");
                    }
                    // Continue visiting the rest of the tree
                    visit_mut::visit_expr_mut(self, node);
                }
            }

            // Apply the visitor to the function block
            let mut visitor = ReturnReplacer;
            visitor.visit_block_mut(&mut input.block);
        }
        let fn_block = &input.block;

        expanded = quote! {
            #[inline(always)]
            #fn_vis fn #fn_name<#fn_generics>(#fn_args, res: *mut #fn_return_type) -> engine::coroutine::CoroutineImpl {
                std::boxed::Box::pin(#[coroutine] static move || {
                    #fn_block
                })
            }
        }
    }

    TokenStream::from(expanded)
}