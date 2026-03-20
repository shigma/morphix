#[allow(unused_imports)]
use morphix::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
pub enum Foo<'i> {
    A(u32),
    B(u32, u32),
    C { bar: &'i mut String },
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, 'i, S: ?Sized, N = ::morphix::helper::Zero>
    where
        &'i mut String: ::morphix::Observe + 'ob,
    {
        ptr: ::morphix::helper::Pointer<S>,
        mutated: bool,
        variant: FooObserverVariant<'ob, 'i>,
        phantom: ::std::marker::PhantomData<&'ob mut N>,
    }
    pub enum FooObserverVariant<'ob, 'i>
    where
        &'i mut String: ::morphix::Observe + 'ob,
    {
        A(::morphix::observe::DefaultObserver<'ob, u32>),
        B(
            ::morphix::observe::DefaultObserver<'ob, u32>,
            ::morphix::observe::DefaultObserver<'ob, u32>,
        ),
        C { bar: ::morphix::observe::DefaultObserver<'ob, &'i mut String> },
        __Unknown,
    }
    impl<'ob, 'i> FooObserverVariant<'ob, 'i>
    where
        &'i mut String: ::morphix::Observe,
    {
        fn observe(value: &mut Foo<'i>) -> Self {
            match value {
                Foo::A(v0) => Self::A(::morphix::observe::Observer::observe(v0)),
                Foo::B(v0, v1) => {
                    Self::B(
                        ::morphix::observe::Observer::observe(v0),
                        ::morphix::observe::Observer::observe(v1),
                    )
                }
                Foo::C { bar } => {
                    Self::C {
                        bar: ::morphix::observe::Observer::observe(bar),
                    }
                }
            }
        }
        unsafe fn relocate(&mut self, value: &mut Foo<'i>) {
            unsafe {
                match (self, value) {
                    (Self::A(u0), Foo::A(v0)) => {
                        ::morphix::observe::Observer::relocate(u0, v0);
                    }
                    (Self::B(u0, u1), Foo::B(v0, v1)) => {
                        ::morphix::observe::Observer::relocate(u0, v0);
                        ::morphix::observe::Observer::relocate(u1, v1);
                    }
                    (Self::C { bar: u0 }, Foo::C { bar: v0 }) => {
                        ::morphix::observe::Observer::relocate(u0, v0);
                    }
                    (Self::__Unknown, _) => {}
                    _ => panic!("inconsistent state for FooObserver"),
                }
            }
        }
        fn flush(&mut self, __value: *const Foo<'i>) -> ::morphix::Mutations
        where
            Foo<'i>: ::morphix::helper::serde::Serialize + 'static,
            ::morphix::observe::DefaultObserver<
                'ob,
                &'i mut String,
            >: ::morphix::observe::SerializeObserver,
        {
            match self {
                Self::A(u0) => {
                    unsafe {
                        ::morphix::observe::SerializeObserver::flush(u0)
                            .with_prefix("data")
                    }
                }
                Self::B(u0, u1) => {
                    let mutations_0 = unsafe {
                        ::morphix::observe::SerializeObserver::flush(u0)
                    };
                    let mutations_1 = unsafe {
                        ::morphix::observe::SerializeObserver::flush(u1)
                    };
                    if mutations_0.is_replace() && mutations_1.is_replace() {
                        return ::morphix::Mutations::replace(unsafe { &*__value });
                    }
                    let mut mutations = ::morphix::Mutations::new()
                        .with_capacity(
                            !mutations_0.is_empty() as usize
                                + !mutations_1.is_empty() as usize,
                        );
                    mutations.insert(0usize, mutations_0);
                    mutations.insert(1usize, mutations_1);
                    mutations.with_prefix("data")
                }
                Self::C { bar } => {
                    let mutations_bar = unsafe {
                        ::morphix::observe::SerializeObserver::flush(bar)
                    };
                    if mutations_bar.is_replace() {
                        return ::morphix::Mutations::replace(unsafe { &*__value });
                    }
                    let mut mutations = ::morphix::Mutations::new()
                        .with_capacity(!mutations_bar.is_empty() as usize);
                    mutations.insert("bar", mutations_bar);
                    mutations.with_prefix("data")
                }
                Self::__Unknown => ::morphix::Mutations::new(),
            }
        }
        fn flat_flush(&mut self, __value: *const Foo<'i>) -> ::morphix::Mutations
        where
            Foo<'i>: ::morphix::helper::serde::Serialize + 'static,
            ::morphix::observe::DefaultObserver<
                'ob,
                &'i mut String,
            >: ::morphix::observe::SerializeObserver,
        {
            match self {
                Self::A(u0) => {
                    unsafe {
                        ::morphix::observe::SerializeObserver::flat_flush(u0)
                            .with_prefix("data")
                    }
                }
                Self::C { bar } => {
                    let mutations_bar = unsafe {
                        ::morphix::observe::SerializeObserver::flush(bar)
                    };
                    let mut mutations = ::morphix::Mutations::new()
                        .with_capacity(!mutations_bar.is_empty() as usize)
                        .with_replace(mutations_bar.is_replace());
                    mutations.insert("bar", mutations_bar);
                    mutations.with_prefix("data")
                }
                _ => panic!("flat_flush can only be called on structs and maps"),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::std::ops::Deref for FooObserver<'ob, 'i, S, N>
    where
        &'i mut String: ::morphix::Observe,
    {
        type Target = ::morphix::helper::Pointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.ptr
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::std::ops::DerefMut for FooObserver<'ob, 'i, S, N>
    where
        &'i mut String: ::morphix::Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.mutated = true;
            self.variant = FooObserverVariant::__Unknown;
            &mut self.ptr
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::morphix::helper::QuasiObserver
    for FooObserver<'ob, 'i, S, N>
    where
        &'i mut String: ::morphix::Observe,
        S: ::morphix::helper::AsDeref<N>,
        N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        type InnerDepth = N;
        fn invalidate(this: &mut Self) {
            this.mutated = true;
            this.variant = FooObserverVariant::__Unknown;
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::morphix::observe::Observer
    for FooObserver<'ob, 'i, S, N>
    where
        &'i mut String: ::morphix::Observe,
        S: ::morphix::helper::AsDerefMut<N, Target = Foo<'i>>,
        N: ::morphix::helper::Unsigned,
    {
        fn observe(head: &mut S) -> Self {
            let __value = head.as_deref_mut();
            Self {
                mutated: false,
                variant: FooObserverVariant::observe(__value),
                ptr: ::morphix::helper::Pointer::new(head),
                phantom: ::std::marker::PhantomData,
            }
        }
        unsafe fn relocate(this: &mut Self, head: &mut S) {
            let __value = head.as_deref_mut();
            unsafe { this.variant.relocate(__value) }
            ::morphix::helper::Pointer::set(this, head);
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::morphix::observe::SerializeObserver
    for FooObserver<'ob, 'i, S, N>
    where
        Foo<'i>: ::morphix::helper::serde::Serialize + 'static,
        &'i mut String: ::morphix::Observe,
        S: ::morphix::helper::AsDeref<N, Target = Foo<'i>>,
        N: ::morphix::helper::Unsigned,
        ::morphix::observe::DefaultObserver<
            'ob,
            &'i mut String,
        >: ::morphix::observe::SerializeObserver,
    {
        unsafe fn flush(this: &mut Self) -> ::morphix::Mutations {
            let value = this.ptr.as_deref();
            if !this.mutated {
                return this.variant.flush(value);
            }
            this.mutated = false;
            this.variant = FooObserverVariant::__Unknown;
            ::morphix::Mutations::replace(this.as_deref())
        }
        unsafe fn flat_flush(this: &mut Self) -> ::morphix::Mutations {
            let value = this.ptr.as_deref();
            if !this.mutated {
                return this.variant.flat_flush(value);
            }
            this.mutated = false;
            this.variant = FooObserverVariant::__Unknown;
            ::morphix::Mutations::replace(this.as_deref())
        }
    }
    #[automatically_derived]
    impl<'i> ::morphix::Observe for Foo<'i>
    where
        Self: ::morphix::helper::serde::Serialize,
        &'i mut String: ::morphix::Observe,
    {
        type Observer<'ob, S, N> = FooObserver<'ob, 'i, S, N>
        where
            Self: 'ob,
            &'i mut String: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
