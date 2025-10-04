use quote::ToTokens;

pub trait SynSub {
    fn attribute(&mut self, _attribute: &mut syn::Attribute) {}

    fn block(&mut self, block: &mut syn::Block) {
        for stmt in &mut block.stmts {
            self.stmt(stmt);
        }
    }

    fn expr(&mut self, expr: &mut syn::Expr) {
        if let Some(new_expr) = match expr {
            syn::Expr::Array(expr_array) => self.expr_array(expr_array),
            syn::Expr::Assign(expr_assign) => self.expr_assign(expr_assign),
            syn::Expr::Async(expr_async) => self.expr_async(expr_async),
            syn::Expr::Await(expr_await) => self.expr_await(expr_await),
            syn::Expr::Binary(expr_binary) => self.expr_binary(expr_binary),
            syn::Expr::Block(expr_block) => self.expr_block(expr_block),
            syn::Expr::Break(expr_break) => self.expr_break(expr_break),
            syn::Expr::Call(expr_call) => self.expr_call(expr_call),
            syn::Expr::Cast(expr_cast) => self.expr_cast(expr_cast),
            syn::Expr::Closure(expr_closure) => self.expr_closure(expr_closure),
            syn::Expr::Const(expr_const) => self.expr_const(expr_const),
            syn::Expr::Continue(expr_continue) => self.expr_continue(expr_continue),
            syn::Expr::Field(expr_field) => self.expr_field(expr_field),
            syn::Expr::ForLoop(expr_for_loop) => self.expr_for_loop(expr_for_loop),
            syn::Expr::Group(expr_group) => self.expr_group(expr_group),
            syn::Expr::If(expr_if) => self.expr_if(expr_if),
            syn::Expr::Index(expr_index) => self.expr_index(expr_index),
            syn::Expr::Let(expr_let) => self.expr_let(expr_let),
            syn::Expr::Lit(expr_lit) => self.expr_lit(expr_lit),
            syn::Expr::Loop(expr_loop) => self.expr_loop(expr_loop),
            syn::Expr::Macro(expr_macro) => self.expr_macro(expr_macro),
            syn::Expr::Match(expr_match) => self.expr_match(expr_match),
            syn::Expr::MethodCall(expr_method_call) => self.expr_method_call(expr_method_call),
            syn::Expr::Paren(expr_paren) => self.expr_paren(expr_paren),
            syn::Expr::Path(expr_path) => self.expr_path(expr_path),
            syn::Expr::Range(expr_range) => self.expr_range(expr_range),
            syn::Expr::RawAddr(expr_raw_addr) => self.expr_raw_addr(expr_raw_addr),
            syn::Expr::Reference(expr_reference) => self.expr_reference(expr_reference),
            syn::Expr::Repeat(expr_repeat) => self.expr_repeat(expr_repeat),
            syn::Expr::Return(expr_return) => self.expr_return(expr_return),
            syn::Expr::Struct(expr_struct) => self.expr_struct(expr_struct),
            syn::Expr::Try(expr_try) => self.expr_try(expr_try),
            syn::Expr::TryBlock(expr_try_block) => self.expr_try_block(expr_try_block),
            syn::Expr::Tuple(expr_tuple) => self.expr_tuple(expr_tuple),
            syn::Expr::Unary(expr_unary) => self.expr_unary(expr_unary),
            syn::Expr::Unsafe(expr_unsafe) => self.expr_unsafe(expr_unsafe),
            syn::Expr::Verbatim(_) => None,
            syn::Expr::While(expr_while) => self.expr_while(expr_while),
            syn::Expr::Yield(expr_yield) => self.expr_yield(expr_yield),
            _ => unimplemented!("unimplemented expr: {}", expr.to_token_stream()),
        } {
            *expr = new_expr;
        }
    }

    fn expr_array(&mut self, expr_array: &mut syn::ExprArray) -> Option<syn::Expr> {
        for attribute in &mut expr_array.attrs {
            self.attribute(attribute);
        }
        for expr in &mut expr_array.elems {
            self.expr(expr);
        }
        None
    }

    fn expr_assign(&mut self, expr_assign: &mut syn::ExprAssign) -> Option<syn::Expr> {
        for attribute in &mut expr_assign.attrs {
            self.attribute(attribute);
        }
        self.expr(&mut expr_assign.left);
        self.expr(&mut expr_assign.right);
        None
    }

    fn expr_async(&mut self, expr_async: &mut syn::ExprAsync) -> Option<syn::Expr> {
        for attribute in &mut expr_async.attrs {
            self.attribute(attribute);
        }
        self.block(&mut expr_async.block);
        None
    }

    fn expr_await(&mut self, expr_await: &mut syn::ExprAwait) -> Option<syn::Expr> {
        for attribute in &mut expr_await.attrs {
            self.attribute(attribute);
        }
        self.expr(&mut expr_await.base);
        None
    }

    fn expr_binary(&mut self, expr_binary: &mut syn::ExprBinary) -> Option<syn::Expr> {
        for attribute in &mut expr_binary.attrs {
            self.attribute(attribute);
        }
        self.expr(&mut expr_binary.left);
        self.expr(&mut expr_binary.right);
        None
    }

    fn expr_block(&mut self, expr_block: &mut syn::ExprBlock) -> Option<syn::Expr> {
        for attribute in &mut expr_block.attrs {
            self.attribute(attribute);
        }
        if let Some(label) = &mut expr_block.label {
            self.lifetime(&mut label.name);
        }
        self.block(&mut expr_block.block);
        None
    }

    fn expr_break(&mut self, expr_break: &mut syn::ExprBreak) -> Option<syn::Expr> {
        if let Some(lifetime) = &mut expr_break.label {
            self.lifetime(lifetime);
        }
        if let Some(expr) = &mut expr_break.expr {
            self.expr(expr);
        }
        None
    }

    fn expr_call(&mut self, expr_call: &mut syn::ExprCall) -> Option<syn::Expr> {
        self.expr(&mut expr_call.func);
        for expr in &mut expr_call.args {
            self.expr(expr);
        }
        None
    }

    fn expr_cast(&mut self, expr_cast: &mut syn::ExprCast) -> Option<syn::Expr> {
        self.expr(&mut expr_cast.expr);
        None
    }

    fn expr_closure(&mut self, expr_closure: &mut syn::ExprClosure) -> Option<syn::Expr> {
        self.expr(&mut expr_closure.body);
        None
    }

    fn expr_const(&mut self, expr_const: &mut syn::ExprConst) -> Option<syn::Expr> {
        self.block(&mut expr_const.block);
        None
    }

    fn expr_continue(&mut self, expr_continue: &mut syn::ExprContinue) -> Option<syn::Expr> {
        if let Some(lifetime) = &mut expr_continue.label {
            self.lifetime(lifetime);
        }
        None
    }

    fn expr_field(&mut self, expr_field: &mut syn::ExprField) -> Option<syn::Expr> {
        self.expr(&mut expr_field.base);
        None
    }

    fn expr_for_loop(&mut self, expr_for_loop: &mut syn::ExprForLoop) -> Option<syn::Expr> {
        if let Some(label) = &mut expr_for_loop.label {
            self.lifetime(&mut label.name);
        }
        self.expr(&mut expr_for_loop.expr);
        self.block(&mut expr_for_loop.body);
        None
    }

    fn expr_group(&mut self, expr_group: &mut syn::ExprGroup) -> Option<syn::Expr> {
        self.expr(&mut expr_group.expr);
        None
    }

    fn expr_if(&mut self, expr_if: &mut syn::ExprIf) -> Option<syn::Expr> {
        self.expr(&mut expr_if.cond);
        self.block(&mut expr_if.then_branch);
        if let Some((_, expr)) = &mut expr_if.else_branch {
            self.expr(expr);
        }
        None
    }

    fn expr_index(&mut self, expr_index: &mut syn::ExprIndex) -> Option<syn::Expr> {
        self.expr(&mut expr_index.expr);
        self.expr(&mut expr_index.index);
        None
    }

    fn expr_let(&mut self, expr_let: &mut syn::ExprLet) -> Option<syn::Expr> {
        self.expr(&mut expr_let.expr);
        None
    }

    fn expr_lit(&mut self, _expr_lit: &mut syn::ExprLit) -> Option<syn::Expr> {
        None
    }

    fn expr_loop(&mut self, expr_loop: &mut syn::ExprLoop) -> Option<syn::Expr> {
        if let Some(label) = &mut expr_loop.label {
            self.lifetime(&mut label.name);
        }
        self.block(&mut expr_loop.body);
        None
    }

    fn expr_macro(&mut self, _expr_macro: &mut syn::ExprMacro) -> Option<syn::Expr> {
        None
    }

    fn expr_match(&mut self, expr_match: &mut syn::ExprMatch) -> Option<syn::Expr> {
        self.expr(&mut expr_match.expr);
        for arm in &mut expr_match.arms {
            self.expr(&mut arm.body);
        }
        None
    }

    fn expr_method_call(&mut self, expr_method_call: &mut syn::ExprMethodCall) -> Option<syn::Expr> {
        self.expr(&mut expr_method_call.receiver);
        for expr in &mut expr_method_call.args {
            self.expr(expr);
        }
        None
    }

    fn expr_paren(&mut self, expr_paren: &mut syn::ExprParen) -> Option<syn::Expr> {
        self.expr(&mut expr_paren.expr);
        None
    }

    fn expr_path(&mut self, _expr_path: &mut syn::ExprPath) -> Option<syn::Expr> {
        None
    }

    fn expr_range(&mut self, expr_range: &mut syn::ExprRange) -> Option<syn::Expr> {
        if let Some(expr) = &mut expr_range.start {
            self.expr(expr);
        }
        if let Some(expr) = &mut expr_range.end {
            self.expr(expr);
        }
        None
    }

    fn expr_raw_addr(&mut self, expr_raw_addr: &mut syn::ExprRawAddr) -> Option<syn::Expr> {
        self.expr(&mut expr_raw_addr.expr);
        None
    }

    fn expr_reference(&mut self, expr_reference: &mut syn::ExprReference) -> Option<syn::Expr> {
        self.expr(&mut expr_reference.expr);
        None
    }

    fn expr_repeat(&mut self, expr_repeat: &mut syn::ExprRepeat) -> Option<syn::Expr> {
        self.expr(&mut expr_repeat.expr);
        self.expr(&mut expr_repeat.len);
        None
    }

    fn expr_return(&mut self, expr_return: &mut syn::ExprReturn) -> Option<syn::Expr> {
        if let Some(expr) = &mut expr_return.expr {
            self.expr(expr);
        }
        None
    }

    fn expr_struct(&mut self, expr_struct: &mut syn::ExprStruct) -> Option<syn::Expr> {
        for field in &mut expr_struct.fields {
            self.expr(&mut field.expr);
        }
        if let Some(expr) = &mut expr_struct.rest {
            self.expr(expr);
        }
        None
    }

    fn expr_try(&mut self, expr_try: &mut syn::ExprTry) -> Option<syn::Expr> {
        self.expr(&mut expr_try.expr);
        None
    }

    fn expr_try_block(&mut self, expr_try_block: &mut syn::ExprTryBlock) -> Option<syn::Expr> {
        self.block(&mut expr_try_block.block);
        None
    }

    fn expr_tuple(&mut self, expr_tuple: &mut syn::ExprTuple) -> Option<syn::Expr> {
        for expr in &mut expr_tuple.elems {
            self.expr(expr);
        }
        None
    }

    fn expr_unary(&mut self, expr_unary: &mut syn::ExprUnary) -> Option<syn::Expr> {
        self.expr(&mut expr_unary.expr);
        None
    }

    fn expr_unsafe(&mut self, expr_unsafe: &mut syn::ExprUnsafe) -> Option<syn::Expr> {
        self.block(&mut expr_unsafe.block);
        None
    }

    fn expr_while(&mut self, expr_while: &mut syn::ExprWhile) -> Option<syn::Expr> {
        if let Some(label) = &mut expr_while.label {
            self.lifetime(&mut label.name);
        }
        self.expr(&mut expr_while.cond);
        self.block(&mut expr_while.body);
        None
    }

    fn expr_yield(&mut self, expr_yield: &mut syn::ExprYield) -> Option<syn::Expr> {
        if let Some(expr) = &mut expr_yield.expr {
            self.expr(expr);
        }
        None
    }

    fn foreign_item(&mut self, foreign_item: &mut syn::ForeignItem) {
        match foreign_item {
            syn::ForeignItem::Fn(foreign_item_fn) => self.foreign_item_fn(foreign_item_fn),
            syn::ForeignItem::Macro(foreign_item_macro) => self.foreign_item_macro(foreign_item_macro),
            syn::ForeignItem::Static(foreign_item_static) => self.foreign_item_static(foreign_item_static),
            syn::ForeignItem::Type(foreign_item_type) => self.foreign_item_type(foreign_item_type),
            syn::ForeignItem::Verbatim(_) => {},
            _ => unimplemented!("unimplemented foreign_item: {}", foreign_item.to_token_stream()),
        }
    }

    fn foreign_item_fn(&mut self, _foreign_item_fn: &mut syn::ForeignItemFn) {}

    fn foreign_item_macro(&mut self, _foreign_item_macro: &mut syn::ForeignItemMacro) {}

    fn foreign_item_static(&mut self, _foreign_item_static: &mut syn::ForeignItemStatic) {}

    fn foreign_item_type(&mut self, _foreign_item_type: &mut syn::ForeignItemType) {}

    fn impl_item(&mut self, impl_item: &mut syn::ImplItem) {
        match impl_item {
            syn::ImplItem::Const(impl_item_const) => self.impl_item_const(impl_item_const),
            syn::ImplItem::Fn(impl_item_fn) => self.impl_item_fn(impl_item_fn),
            syn::ImplItem::Macro(impl_item_macro) => self.impl_item_macro(impl_item_macro),
            syn::ImplItem::Type(impl_item_type) => self.impl_item_type(impl_item_type),
            syn::ImplItem::Verbatim(_) => {},
            _ => unimplemented!("unimplemented impl_item: {}", impl_item.to_token_stream()),
        }
    }

    fn impl_item_const(&mut self, _impl_item_const: &mut syn::ImplItemConst) {}

    fn impl_item_fn(&mut self, impl_item_fn: &mut syn::ImplItemFn) {
        self.block(&mut impl_item_fn.block);
    }

    fn impl_item_macro(&mut self, _impl_item_macro: &mut syn::ImplItemMacro) {}

    fn impl_item_type(&mut self, _impl_item_type: &mut syn::ImplItemType) {}

    fn item(&mut self, item: &mut syn::Item) {
        match item {
            syn::Item::Const(item_const) => self.item_const(item_const),
            syn::Item::Enum(item_enum) => self.item_enum(item_enum),
            syn::Item::ExternCrate(item_extern_crate) => self.item_extern_crate(item_extern_crate),
            syn::Item::Fn(item_fn) => self.item_fn(item_fn),
            syn::Item::ForeignMod(item_foreign_mod) => self.item_foreign_mod(item_foreign_mod),
            syn::Item::Impl(item_impl) => self.item_impl(item_impl),
            syn::Item::Macro(item_macro) => self.item_macro(item_macro),
            syn::Item::Mod(item_mod) => self.item_mod(item_mod),
            syn::Item::Static(item_static) => self.item_static(item_static),
            syn::Item::Struct(item_struct) => self.item_struct(item_struct),
            syn::Item::Trait(item_trait) => self.item_trait(item_trait),
            syn::Item::TraitAlias(item_trait_alias) => self.item_trait_alias(item_trait_alias),
            syn::Item::Type(item_type) => self.item_type(item_type),
            syn::Item::Union(item_union) => self.item_union(item_union),
            syn::Item::Use(item_use) => self.item_use(item_use),
            syn::Item::Verbatim(_) => {},
            _ => unimplemented!("unimplemented item: {}", item.to_token_stream()),
        }
    }

    fn item_const(&mut self, item_const: &mut syn::ItemConst) {
        self.expr(&mut item_const.expr);
    }

    fn item_enum(&mut self, _item_enum: &mut syn::ItemEnum) {}

    fn item_extern_crate(&mut self, _item_extern_crate: &mut syn::ItemExternCrate) {}

    fn item_fn(&mut self, item_fn: &mut syn::ItemFn) {
        self.block(&mut item_fn.block);
    }

    fn item_foreign_mod(&mut self, item_foreign_mod: &mut syn::ItemForeignMod) {
        for foreign_item in &mut item_foreign_mod.items {
            self.foreign_item(foreign_item);
        }
    }

    fn item_impl(&mut self, item_impl: &mut syn::ItemImpl) {
        for impl_item in &mut item_impl.items {
            self.impl_item(impl_item);
        }
    }

    fn item_macro(&mut self, _item_macro: &mut syn::ItemMacro) {}

    fn item_mod(&mut self, item_mod: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut item_mod.content {
            for item in items {
                self.item(item);
            }
        }
    }

    fn item_static(&mut self, item_static: &mut syn::ItemStatic) {
        self.expr(&mut item_static.expr);
    }

    fn item_struct(&mut self, _item_struct: &mut syn::ItemStruct) {}

    fn item_trait(&mut self, item_trait: &mut syn::ItemTrait) {
        for trait_item in &mut item_trait.items {
            self.trait_item(trait_item);
        }
    }

    fn item_trait_alias(&mut self, _item_trait_alias: &mut syn::ItemTraitAlias) {}

    fn item_type(&mut self, _item_type: &mut syn::ItemType) {}

    fn item_union(&mut self, _item_union: &mut syn::ItemUnion) {}

    fn item_use(&mut self, _item_use: &mut syn::ItemUse) {}

    fn lifetime(&mut self, _lifetime: &mut syn::Lifetime) {}

    fn local(&mut self, local: &mut syn::Local) {
        if let Some(local_init) = &mut local.init {
            self.expr(&mut local_init.expr);
            if let Some((_, expr)) = &mut local_init.diverge {
                self.expr(expr);
            }
        }
    }

    fn stmt(&mut self, stmt: &mut syn::Stmt) {
        match stmt {
            syn::Stmt::Local(local) => self.local(local),
            syn::Stmt::Expr(expr, ..) => self.expr(expr),
            syn::Stmt::Item(item) => self.item(item),
            syn::Stmt::Macro(stmt_macro) => self.stmt_macro(stmt_macro),
            #[allow(unreachable_patterns)]
            _ => unimplemented!("unimplemented stmt: {}", stmt.to_token_stream()),
        }
    }

    fn stmt_macro(&mut self, _stmt_macro: &mut syn::StmtMacro) {}

    fn trait_item(&mut self, trait_item: &mut syn::TraitItem) {
        match trait_item {
            syn::TraitItem::Const(trait_item_const) => self.trait_item_const(trait_item_const),
            syn::TraitItem::Fn(trait_item_fn) => self.trait_item_fn(trait_item_fn),
            syn::TraitItem::Macro(trait_item_macro) => self.trait_item_macro(trait_item_macro),
            syn::TraitItem::Type(trait_item_type) => self.trait_item_type(trait_item_type),
            syn::TraitItem::Verbatim(_) => {},
            _ => unimplemented!("unimplemented trait_item: {}", trait_item.to_token_stream()),
        }
    }

    fn trait_item_const(&mut self, _trait_item_const: &mut syn::TraitItemConst) {}

    fn trait_item_fn(&mut self, trait_item_fn: &mut syn::TraitItemFn) {
        if let Some(default) = &mut trait_item_fn.default {
            self.block(default);
        }
    }

    fn trait_item_macro(&mut self, _trait_item_macro: &mut syn::TraitItemMacro) {}

    fn trait_item_type(&mut self, _trait_item_type: &mut syn::TraitItemType) {}
}
