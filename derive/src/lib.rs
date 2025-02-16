use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};

#[proc_macro_derive(Observe)]
pub fn derive_observe(input: TokenStream) -> TokenStream {
    let derive: syn::DeriveInput = syn::parse_macro_input!(input);
    let ident = &derive.ident;
    let (impl_generics, type_generics, where_clause) = derive.generics.split_for_impl();
    let ident_ob = format_ident!("{}Ob", ident);
    let mut type_fields = vec![];
    let mut inst_fields = vec![];
    match &derive.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => {
            for name in named {
                let ident = name.ident.as_ref().unwrap();
                let ty = &name.ty;
                type_fields.push(quote! {
                    pub #ident: ::umili::Ob<'i, #ty>,
                });
                inst_fields.push(quote! {
                    #ident: ::umili::Ob {
                        value: &mut self.#ident,
                        ctx: ctx.extend(stringify!(#ident)),
                    },
                });
            }
        },
        _ => unimplemented!("not implemented"),
    };
    quote! {
        #[automatically_derived]
        impl #impl_generics Observe for #ident #type_generics #where_clause {
            type Target<'i> = #ident_ob<'i>;

            fn observe(&mut self, ctx: &::umili::Context) -> Self::Target<'_> {
                #ident_ob {
                    #(#inst_fields)*
                }
            }
        }

        pub struct #ident_ob<'i> {
            #(#type_fields)*
        }
    }.into()
}

#[proc_macro]
pub fn observe(input: TokenStream) -> TokenStream {
    let input: syn::Expr = syn::parse_macro_input!(input);
    let syn::Expr::Closure(mut closure) = input else {
        panic!("expect a closure expression")
    };
    if closure.inputs.len() != 1 {
        panic!("expect a closure with one argument")
    }
    let syn::Pat::Ident(syn::PatIdent { ident, .. }) = &closure.inputs[0] else {
        panic!("expect a closure with one argument")
    };
    let body = &mut closure.body;
    let body_shadow = body.to_token_stream();
    subst_expr(body, ident);
    quote! {
        {
            use ::std::ops::*;
            let _ = || #body_shadow;
            let ctx = ::umili::Context::new();
            let mut #ident = #ident.observe(&ctx);
            #body;
            ctx.collect()
        }
    }.into()
}

fn subst_expr_field(expr_field: &mut syn::ExprField, ident: &syn::Ident, inner: bool) -> Option<syn::Expr> {
    // erase span info from expr_field
    let member = format_ident!("{}", expr_field.member.to_token_stream().to_string());
    let method = match inner {
        true => format_ident!("borrow"),
        false => format_ident!("borrow_mut"),
    };
    match &mut *expr_field.base {
        syn::Expr::Path(expr_path) => {
            if expr_path.to_token_stream().to_string() == ident.to_string() {
                return Some(syn::parse_quote! {
                    #ident.#member.#method()
                });
            }
        },
        syn::Expr::Field(expr_field) => {
            if let Some(new_expr) = subst_expr_field(expr_field, ident, true) {
                return Some(syn::parse_quote! {
                    #new_expr.#member.#method()
                });
            }
        },
        _ => subst_expr(&mut expr_field.base, ident),
    }
    None
}

fn subst_expr(expr: &mut syn::Expr, ident: &syn::Ident) {
    match expr {
        syn::Expr::Array(expr_array) => {
            for expr in expr_array.elems.iter_mut() {
                subst_expr(expr, ident);
            }
        },
        syn::Expr::Assign(expr_assign) => {
            subst_expr(&mut expr_assign.left, ident);
            subst_expr(&mut expr_assign.right, ident);
        },
        syn::Expr::Async(expr_async) => {
            subst_block(&mut expr_async.block, ident);
        },
        syn::Expr::Await(expr_await) => {
            subst_expr(&mut expr_await.base, ident);
        },
        syn::Expr::Binary(expr_binary) => {
            subst_expr(&mut expr_binary.left, ident);
            subst_expr(&mut expr_binary.right, ident);
            match &expr_binary.op {
                syn::BinOp::AddAssign(..) => {
                    let left = &expr_binary.left;
                    let right = &expr_binary.right;
                    *expr = syn::parse_quote! {
                        #left.add_assign(#right)
                    }
                },
                _ => {},
            }
        },
        syn::Expr::Block(expr_block) => {
            subst_block(&mut expr_block.block, ident);
        },
        syn::Expr::Break(..) => {},
        syn::Expr::Call(expr_call) => {
            subst_expr(&mut expr_call.func, ident);
            for expr in expr_call.args.iter_mut() {
                subst_expr(expr, ident);
            }
        },
        syn::Expr::Cast(expr_cast) => {
            subst_expr(&mut expr_cast.expr, ident);
        },
        syn::Expr::Closure(expr_closure) => {
            subst_expr(&mut expr_closure.body, ident);
        },
        syn::Expr::Const(expr_const) => {
            subst_block(&mut expr_const.block, ident);
        },
        syn::Expr::Continue(..) => {},
        syn::Expr::Field(expr_field) => {
            if let Some(new_expr) = subst_expr_field(expr_field, ident, false) {
                *expr = new_expr;
            }
        },
        syn::Expr::ForLoop(expr_for_loop) => {
            subst_expr(&mut expr_for_loop.expr, ident);
            subst_block(&mut expr_for_loop.body, ident);
        },
        syn::Expr::Group(expr_group) => {
            subst_expr(&mut expr_group.expr, ident);
        },
        syn::Expr::If(expr_if) => {
            subst_expr(&mut expr_if.cond, ident);
            subst_block(&mut expr_if.then_branch, ident);
            if let Some((_, expr)) = &mut expr_if.else_branch {
                subst_expr(expr, ident);
            }
        },
        syn::Expr::Index(expr_index) => {
            subst_expr(&mut expr_index.expr, ident);
            subst_expr(&mut expr_index.index, ident);
        },
        syn::Expr::Let(expr_let) => {
            subst_expr(&mut expr_let.expr, ident);
        },
        syn::Expr::Lit(..) => {},
        syn::Expr::Loop(expr_loop) => {
            subst_block(&mut expr_loop.body, ident);
        },
        syn::Expr::Macro(..) => {},
        syn::Expr::Match(expr_match) => {
            subst_expr(&mut expr_match.expr, ident);
            for arm in expr_match.arms.iter_mut() {
                subst_expr(&mut arm.body, ident);
            }
        },
        syn::Expr::MethodCall(expr_method_call) => {
            subst_expr(&mut expr_method_call.receiver, ident);
            for expr in expr_method_call.args.iter_mut() {
                subst_expr(expr, ident);
            }
        },
        syn::Expr::Paren(expr_paren) => {
            subst_expr(&mut expr_paren.expr, ident);
        },
        syn::Expr::Path(..) => {},
        syn::Expr::Range(expr_range) => {
            if let Some(expr) = &mut expr_range.start {
                subst_expr(expr, ident);
            }
            if let Some(expr) = &mut expr_range.end {
                subst_expr(expr, ident);
            }
        },
        syn::Expr::RawAddr(expr_raw_addr) => {
            subst_expr(&mut expr_raw_addr.expr, ident);
        },
        syn::Expr::Reference(expr_reference) => {
            subst_expr(&mut expr_reference.expr, ident);
        },
        syn::Expr::Repeat(expr_repeat) => {
            subst_expr(&mut expr_repeat.expr, ident);
            subst_expr(&mut expr_repeat.len, ident);
        },
        syn::Expr::Return(expr_return) => {
            if let Some(expr) = &mut expr_return.expr {
                subst_expr(expr, ident);
            }
        },
        syn::Expr::Struct(expr_struct) => {
            for field in expr_struct.fields.iter_mut() {
                subst_expr(&mut field.expr, ident);
            }
        },
        syn::Expr::Try(expr_try) => {
            subst_expr(&mut expr_try.expr, ident);
        },
        syn::Expr::TryBlock(expr_try_block) => {
            subst_block(&mut expr_try_block.block, ident);
        },
        syn::Expr::Tuple(expr_tuple) => {
            for expr in expr_tuple.elems.iter_mut() {
                subst_expr(expr, ident);
            }
        },
        syn::Expr::Unary(expr_unary) => {
            subst_expr(&mut expr_unary.expr, ident);
        },
        syn::Expr::Unsafe(expr_unsafe) => {
            subst_block(&mut expr_unsafe.block, ident);
        },
        syn::Expr::Verbatim(..) => {},
        syn::Expr::While(expr_while) => {
            subst_expr(&mut expr_while.cond, ident);
            subst_block(&mut expr_while.body, ident);
        },
        syn::Expr::Yield(expr_yield) => {
            if let Some(expr) = &mut expr_yield.expr {
                subst_expr(expr, ident);
            }
        },
        _ => unimplemented!("unimplemented expr: {}", expr.to_token_stream()),
    }
}

fn subst_block(block: &mut syn::Block, ident: &syn::Ident) {
    for stmt in block.stmts.iter_mut() {
        subst_stmt(stmt, ident);
    }
}

fn subst_stmt(stmt: &mut syn::Stmt, ident: &syn::Ident) {
    match stmt {
        syn::Stmt::Local(local) => {
            if let Some(local_init) = &mut local.init {
                subst_expr(&mut local_init.expr, ident);
                if let Some((_, expr)) = &mut local_init.diverge {
                    subst_expr(expr, ident);
                }
            }
        },
        syn::Stmt::Expr(expr, ..) => {
            subst_expr(expr, ident);
        },
        syn::Stmt::Macro(..) => {},
        _ => unimplemented!("unimplemented stmt: {}", stmt.to_token_stream()),
    }
}
