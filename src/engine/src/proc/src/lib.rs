#![feature(coroutines)]
#![feature(coroutine_trait)]
#![feature(stmt_expr_attributes)]
extern crate proc_macro;
use proc_macro::TokenStream;
use std::ops::DerefMut;
use quote::{quote};
use syn::{parse_macro_input, ItemFn, ReturnType, Expr, Stmt, Block};
use syn::token::Semi;

/// Transforms function body. Replaces all `yield` expressions to
/// ```ignore
/// unsafe {
///     let mut coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT = std::mem::MaybeUninit::uninit();
///     #yield_ex;
///     coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT.assume_init()#semi
/// }
/// ```
fn transform_function_yield(block: &mut Block) {
    /// Here we get expr like `yield stream.read()`
    /// and transform it to
    /// ```ignore
    /// unsafe {
    ///     let coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT = std::mem::MaybeUninit::uninit();
    ///     yield stream.read(coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT.as_mut_ptr());
    ///     coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT.assume_init()
    /// }
    /// ```
    fn transform_expr(expr: &mut Expr, semi: Option<Semi>) {
        match expr {
            Expr::Yield(yield_ex) => {
                let mut new_yield_ex = yield_ex.clone().expr.expect("empty yield expression");
                match new_yield_ex.deref_mut() {
                    Expr::Call(call_ex) => {
                        for arg in &mut call_ex.args {
                            transform_expr(arg, None);
                        }
                        call_ex.args.push(syn::parse_quote!(coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT.as_mut_ptr()));
                    },
                    Expr::MethodCall(method_call_ex) => {
                        for arg in &mut method_call_ex.args {
                            transform_expr(arg, None);
                        }
                        method_call_ex.args.push(syn::parse_quote!(coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT.as_mut_ptr()));
                    },
                    _ => panic!("yield expression must call a function or call a method"),
                }

                yield_ex.expr = Some(new_yield_ex);
                let new_expr = syn::parse_quote!(
                    unsafe {
                        let mut coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT = std::mem::MaybeUninit::uninit();
                        #yield_ex;
                        coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT.assume_init()#semi
                    }
                );
                *expr = new_expr;
            }
            Expr::Let(let_ex) => {
                transform_expr(let_ex.expr.deref_mut(), None);
            }
            Expr::Assign(assign_ex) => {
                let right = &mut assign_ex.right;
                transform_expr(right, None);
            }
            Expr::If(if_ex) => {
                transform_expr(if_ex.cond.deref_mut(), None);
                transform_function_yield(&mut if_ex.then_branch);
                if let Some(ref mut else_branch) = if_ex.else_branch {
                    transform_expr(else_branch.1.deref_mut(), None);
                }
            }
            Expr::Block(block_ex) => {
                transform_function_yield(&mut block_ex.block);
            }
            Expr::Return(ret_ex) => {
                if let Some(ref mut expr) = ret_ex.expr {
                    transform_expr(expr, None);
                }
            }
            Expr::Match(match_ex) => {
                transform_expr(match_ex.expr.deref_mut(), None);
                for arm in &mut match_ex.arms {
                    transform_expr(arm.body.deref_mut(), None);
                }
            }
            Expr::Lit(_lit_ex) => {
                // doesn't contain `yield`
            }
            Expr::Paren(paren_ex) => {
                transform_expr(paren_ex.expr.deref_mut(), None);
            }
            Expr::Tuple(tuple_ex) => {
                for mut elem in &mut tuple_ex.elems {
                    transform_expr(elem.deref_mut(), None);
                }
            }
            Expr::Reference(ref_ex) => {
                transform_expr(ref_ex.expr.deref_mut(), None);
            }
            Expr::Closure(_closure_ex) => {
                // closure must not have yield, so we don't need to transform it
            }
            Expr::Field(field_ex) => {
                transform_expr(field_ex.base.deref_mut(), None);
            }
            Expr::MethodCall(method_call_ex) => {
                transform_expr(method_call_ex.receiver.deref_mut(), None);
                for arg in &mut method_call_ex.args {
                    transform_expr(arg, None);
                }
            }
            Expr::Call(call_ex) => {
                for arg in &mut call_ex.args {
                transform_expr(arg, None);
            }
            }
            Expr::Array(array_ex) => {
                for mut elem in &mut array_ex.elems {
                    transform_expr(elem.deref_mut(), None);
                }
            }
            Expr::Cast(cast_ex) => {
                transform_expr(cast_ex.expr.deref_mut(), None);
            }
            Expr::Struct(struct_ex) => {
                // we needn't transform qself, because it doesn't contain yield

                // Do we need rest?
                if let Some(ref mut rest) = struct_ex.rest {
                    transform_expr(rest.deref_mut(), None);
                }
                for field in &mut struct_ex.fields {
                    transform_expr(&mut field.expr, None);
                }
            }
            Expr::Repeat(repeat_ex) => {
                transform_expr(repeat_ex.expr.deref_mut(), None);
                transform_expr(repeat_ex.len.deref_mut(), None);
            }
            Expr::Unary(unary_ex) => {
                transform_expr(unary_ex.expr.deref_mut(), None);
            }
            Expr::Binary(binary_ex) => {
                transform_expr(binary_ex.right.deref_mut(), None);
                transform_expr(binary_ex.left.deref_mut(), None);
            }
            Expr::Unsafe(unsafe_ex) => {
                transform_function_yield(&mut unsafe_ex.block);
            }
            Expr::ForLoop(for_loop_ex) => {
                transform_expr(for_loop_ex.expr.deref_mut(), None);
                transform_function_yield(&mut for_loop_ex.body);
            }
            Expr::Index(index_ex) => {
                transform_expr(index_ex.expr.deref_mut(), None);
                transform_expr(index_ex.index.deref_mut(), None);
            }
            Expr::Loop(loop_ex) => {
                transform_function_yield(&mut loop_ex.body);
            }
            Expr::TryBlock(try_block_ex) => {
                transform_function_yield(&mut try_block_ex.block);
            }
            Expr::While(while_ex) => {
                transform_expr(while_ex.cond.deref_mut(), None);
                transform_function_yield(&mut while_ex.body);
            }
            Expr::Range(range_ex) => {
                if let Some(ref mut end) = range_ex.end {
                    transform_expr(end, None);
                }
                if let Some(ref mut start) = range_ex.start {
                    transform_expr(start, None);
                }
            }
            Expr::Try(_try_ex) => {
                panic!("macro coro does not support try-expressions (?) yet");
            }
            _ => {},
        }
    }

    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Expr(expr, semi) => {
                transform_expr(expr, semi.clone());
            }
            Stmt::Local(local) => {
                if let Some(expr) = &mut local.init {
                    transform_expr(expr.expr.deref_mut(), None);
                }
            }
            _ => {},
        }
    }
}

/// Transforms function body. Replaces all `return` expressions and the implicit return to
/// ```ignore
/// {
///     unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #ret_expr; }
///     return;
/// }
/// ```
fn transform_function_return(block: &mut Block, level: usize) {
    fn transform_expr(expr: &mut Expr, semi: Option<Semi>, level: usize) {
        match expr {
            // TODO Try and range
            Expr::If(if_ex) => {
                transform_function_return(&mut if_ex.then_branch, level);
                if let Some(else_branch) = &mut if_ex.else_branch {
                    transform_expr(&mut else_branch.1, None, level);
                }
            }
            Expr::Block(ref mut block_ex) => {
                transform_function_return(&mut block_ex.block, level);
            }
            Expr::Return(ret_ex) => {
                let ret_expr = ret_ex.clone().expr.unwrap();
                let new_expr: Expr = syn::parse_quote!(
                    {
                        unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #ret_expr; }
                        return;
                    }
                );
                *expr = new_expr;
            }
            Expr::Match(match_ex) => {
                for arm in &mut match_ex.arms {
                    transform_expr(&mut arm.body, None, level + 1);
                }
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #match_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Lit(lit_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #lit_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Paren(paren_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #paren_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Tuple(tuple_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #tuple_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Reference(ref_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #ref_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Closure(closure_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #closure_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Field(field_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #field_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::MethodCall(method_call_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #method_call_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Call(call_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #call_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Array(array_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #array_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Cast(cast_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #cast_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Struct(struct_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #struct_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Repeat(repeat_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #repeat_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Unary(unary_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #unary_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Binary(binary_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #binary_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Unsafe(unsafe_ex) => {
                transform_function_return(&mut unsafe_ex.block, level);
            }
            Expr::Yield(yield_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #yield_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Loop(loop_ex) => {
                transform_function_return(&mut loop_ex.body, level);
            }
            Expr::ForLoop(for_loop_ex) => {
                transform_function_return(&mut for_loop_ex.body, level);
            }
            Expr::While(while_ex) => {
                transform_function_return(&mut while_ex.body, level);
            }
            Expr::TryBlock(try_block_ex) => {
                transform_function_return(&mut try_block_ex.block, level);
            }
            Expr::Range(range_ex) => {
                if level == 1 && semi.is_none() {
                    let new_expr: Expr = syn::parse_quote!(unsafe { *coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT = #range_ex; return;});
                    *expr = new_expr;
                }
            }
            Expr::Try(_try_ex) => {
                panic!("macro coro does not support try-expressions (?) yet");
            }
            _ => {},
        }
    }

    for stmt in &mut block.stmts {
        match stmt {
            Stmt::Expr(expr, semi) => {
                transform_expr(expr, semi.clone(), level);
            }
            Stmt::Local(local) => {
                if let Some(ref mut expr) = local.init {
                    transform_expr(&mut expr.expr, None, 1000);
                }
            }
            _ => {},
        }
    }
}

/// A macro that converts a function into a coroutine creator.
/// In general, we can say that this macro allows the user to write coroutine creators as "usual" functions.
///
/// # Differences coroutine creators from the "usual" functions
///
/// - You can't use try-expressions (?).
/// - You can't use `yield` in the arguments of the macros.
/// - It is better to use `return` keyword instead of implicit return.
/// - These functions are not async.
/// - These functions always returns [`CoroutineImpl`].
///
/// # What this macro does
///
/// - Converts a function into a coroutine creator.
/// - Hide from the user returning via pointers.
/// - Allows the user use statement `yield expr` and hide from the user works with pointers.
///
/// # Limitations
///
/// You can't use `yield` in the arguments of the macros, and you can't use try-expressions (?).
///
/// # Safety
///
/// This macro will panic if the block of code has try-expressions (?) or any all errors.
///
/// # About implicit return
///
/// I tried to add a support of the implicit return, but I don't sure that I process all cases correctly.
/// In my tests, it works well with literals, matches, if statements, calls methods and functions, blocks of code and unsafe blocks.
/// But you always can try, in the worst case it just will not compile.
#[proc_macro_attribute]
pub fn coro(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_args = &input.sig.inputs;
    let fn_vis = &input.vis;
    let fn_generics = &input.sig.generics;
    let fn_return_type = &input.sig.output;
    let fn_where_clause = &input.sig.generics.where_clause;
    let mut fn_block = &mut input.block;
    let attr = &input.attrs;

    transform_function_yield(&mut fn_block);

    let fn_args = match fn_return_type {
        ReturnType::Type(_, t) => {
            transform_function_return(&mut fn_block, 1);
            if fn_args.len() > 0 {
                quote! {
                    #fn_args, coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT: *mut #t
                }
            } else {
                quote! {
                    coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT: *mut #t
                }
            }
        },
        ReturnType::Default => {
            if fn_args.len() > 0 {
                quote! {
                    #fn_args, coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT: *mut ()
                }
            } else {
                quote! {
                    coroutine_argument_DONT_NAME_YOUR_VARIABLE_AS_IT: *mut ()
                }
            }
        },
    };

    let mut expanded;
    expanded = quote! {
        #[inline(always)]
    };
    for attr in attr {
        expanded = quote! {
            #expanded
            #attr
        };
    }

    expanded = quote! {
        #expanded
        #fn_vis fn #fn_name #fn_generics (#fn_args) #fn_where_clause -> engine::coroutine::CoroutineImpl {
            std::boxed::Box::pin(#[coroutine] static move || {
            #fn_block
            })
        }
    };

    TokenStream::from(expanded)
}

/// Wait for the coroutine to finish. It will return the result of the coroutine.
///
/// # Example
///
/// ```ignore
/// use engine::{coro, wait};
/// use engine::net::TcpStream;
/// use engine::buf::Buffer;
/// use std::io::Error;
///
/// #[coro]
/// fn difficult_write(mut stream: TcpStream, mut buf: Buffer) -> usize {
///     loop {
///         let res: Result<Option<Buffer>, Error> = yield stream.write(buf);
///         if res.is_err() {
///             println!("write failed, reason: {}", res.err().unwrap());
///             break;
///         }
///         if let Some(new_buf) = res.unwrap() {
///             buf = new_buf;
///         } else {
///             break;
///         }
///     }
///     42
/// }
///
/// #[coro]
/// fn handle_tcp_stream(mut stream: TcpStream) {
///     let res = wait!(difficult_write(stream, engine::buf::buffer()));
///     println!("{}", res); // 42
/// }
/// ```
#[proc_macro]
pub fn wait(input: TokenStream) -> TokenStream {
    let input_expr = parse_macro_input!(input as Expr);

    let modified_expr = match input_expr {
        Expr::Call(mut call_expr) => {
            call_expr.args.push(syn::parse_quote!(coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT.as_mut_ptr()));
            Expr::Call(call_expr)
        }
        Expr::MethodCall(mut method_call_expr) => {
            method_call_expr.args.push(syn::parse_quote!(coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT.as_mut_ptr()));
            Expr::MethodCall(method_call_expr)
        }
        _ => panic!("The macro only supports function or method calls"),
    };

    let block = quote! {
        unsafe {
            let mut coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT = std::mem::MaybeUninit::uninit();
            let mut coroutine = #modified_expr;
            loop {
                match coroutine.as_mut().resume(()) {
                    std::ops::CoroutineState::Yielded(state) => {
                        yield state;
                    },
                    std::ops::CoroutineState::Complete(res) => break res,
                }
            }
            coroutine_result_DONT_NAME_YOUR_VARIABLE_AS_IT.assume_init()
        }
    };

    TokenStream::from(block)
}

/// Spawn a new coroutine in the local scheduler.
///
/// # Example
///
/// ```ignore
/// use engine::{coro, spawn_local};
/// use engine::net::{TcpListener, TcpStream};
/// use std::net::ToSocketAddrs;
///
/// #[coro]
/// fn run_server() {
///     let mut listener = yield TcpListener::new("engine:8081".to_socket_addrs().unwrap().next().unwrap());
///     loop {
///         let stream = (yield listener.accept()).expect("accept failed");
///         spawn_local!(handle_tcp_stream(stream)); // spawn a new coroutine.
///         // So it will be executed in the local scheduler independently from this coroutine.
///     }
/// }
///
/// #[coro]
/// fn handle_tcp_stream(mut stream: TcpStream) {
///     // process stream
/// }
/// ```
#[proc_macro]
pub fn spawn_local(input: TokenStream) -> TokenStream {
    let input_expr = parse_macro_input!(input as Expr);

    let modified_expr = match input_expr {
        // TODO: think about. Maybe we need to hande expr, that is a CoroutineImpl?
        Expr::Call(mut call_expr) => {
            call_expr.args.push(syn::parse_quote!(std::ptr::null_mut()));
            Expr::Call(call_expr)
        }
        Expr::MethodCall(mut method_call_expr) => {
            method_call_expr.args.push(syn::parse_quote!(std::ptr::null_mut()));
            Expr::MethodCall(method_call_expr)
        }
        _ => panic!("The macro only supports function or method calls"),
    };

    let block = quote! {
        engine::local_scheduler().sched(#modified_expr);
    };

    TokenStream::from(block)
}
