#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Foo<S, T, U>
where
    T: Clone,
{
    A(S),
    B(u32, U),
    #[serde(rename_all = "UPPERCASE")]
    #[serde(rename = "OwO")]
    C { #[serde(skip)] bar: Option<T>, #[serde(rename = "QwQ")] qux: Qux },
    D,
    E(),
    F {},
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, S, T, U, _S: ?Sized, N = ::morphix::helper::Zero>
    where
        T: Clone,
        U: ::morphix::Observe + 'ob,
    {
        ptr: ::morphix::helper::Pointer<_S>,
        mutated: bool,
        initial: FooObserverInitial,
        variant: FooObserverVariant<'ob, S, T, U>,
        phantom: ::std::marker::PhantomData<&'ob mut N>,
    }
    #[derive(Clone, Copy)]
    pub enum FooObserverInitial {
        D,
        E,
        F,
        __Unknown,
    }
    impl FooObserverInitial {
        fn new<S, T, U>(value: &Foo<S, T, U>) -> Self
        where
            T: Clone,
        {
            match value {
                Foo::D => FooObserverInitial::D,
                Foo::E() => FooObserverInitial::E,
                Foo::F {} => FooObserverInitial::F,
                _ => FooObserverInitial::__Unknown,
            }
        }
    }
    pub enum FooObserverVariant<'ob, S, T, U>
    where
        T: Clone,
        U: ::morphix::Observe + 'ob,
    {
        A(::morphix::helper::Pointer<S>),
        B(
            ::morphix::observe::DefaultObserver<'ob, u32>,
            ::morphix::observe::DefaultObserver<'ob, U>,
        ),
        C {
            bar: ::morphix::helper::Pointer<Option<T>>,
            qux: ::morphix::observe::DefaultObserver<'ob, Qux>,
        },
        __Unknown,
    }
    impl<'ob, S, T, U> FooObserverVariant<'ob, S, T, U>
    where
        T: Clone,
        U: ::morphix::Observe,
    {
        fn observe(value: &mut Foo<S, T, U>) -> Self {
            match value {
                Foo::A(v0) => Self::A(::morphix::helper::Pointer::new(v0)),
                Foo::B(v0, v1) => {
                    Self::B(
                        ::morphix::observe::Observer::observe(v0),
                        ::morphix::observe::Observer::observe(v1),
                    )
                }
                Foo::C { bar, qux } => {
                    Self::C {
                        bar: ::morphix::helper::Pointer::new(bar),
                        qux: ::morphix::observe::Observer::observe(qux),
                    }
                }
                _ => Self::__Unknown,
            }
        }
        unsafe fn relocate(&mut self, value: &mut Foo<S, T, U>) {
            unsafe {
                match (self, value) {
                    (Self::A(u0), Foo::A(v0)) => {
                        ::morphix::helper::Pointer::set(u0, v0);
                    }
                    (Self::B(u0, u1), Foo::B(v0, v1)) => {
                        ::morphix::observe::Observer::relocate(u0, v0);
                        ::morphix::observe::Observer::relocate(u1, v1);
                    }
                    (Self::C { bar: u0, qux: u1 }, Foo::C { bar: v0, qux: v1 }) => {
                        ::morphix::helper::Pointer::set(u0, v0);
                        ::morphix::observe::Observer::relocate(u1, v1);
                    }
                    (Self::__Unknown, _) => {}
                    _ => panic!("inconsistent state for FooObserver"),
                }
            }
        }
        fn flush(&mut self, __value: *const Foo<S, T, U>) -> ::morphix::Mutations
        where
            Foo<S, T, U>: ::morphix::helper::serde::Serialize + 'static,
            ::morphix::observe::DefaultObserver<
                'ob,
                U,
            >: ::morphix::observe::SerializeObserver,
        {
            match self {
                Self::A(_) => ::morphix::Mutations::new(),
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
                    mutations.with_prefix("b")
                }
                Self::C { qux, .. } => {
                    let mutations_qux = unsafe {
                        ::morphix::observe::SerializeObserver::flush(qux)
                    };
                    if mutations_qux.is_replace() {
                        return ::morphix::Mutations::replace(unsafe { &*__value });
                    }
                    let mut mutations = ::morphix::Mutations::new()
                        .with_capacity(!mutations_qux.is_empty() as usize);
                    mutations.insert("QwQ", mutations_qux);
                    mutations.with_prefix("OwO")
                }
                Self::__Unknown => ::morphix::Mutations::new(),
            }
        }
        fn flat_flush(&mut self, __value: *const Foo<S, T, U>) -> ::morphix::Mutations
        where
            Foo<S, T, U>: ::morphix::helper::serde::Serialize + 'static,
            ::morphix::observe::DefaultObserver<
                'ob,
                U,
            >: ::morphix::observe::SerializeObserver,
        {
            match self {
                Self::A(_) => ::morphix::Mutations::new(),
                Self::C { qux, .. } => {
                    let mutations_qux = unsafe {
                        ::morphix::observe::SerializeObserver::flush(qux)
                    };
                    let mut mutations = ::morphix::Mutations::new()
                        .with_capacity(!mutations_qux.is_empty() as usize)
                        .with_replace(mutations_qux.is_replace());
                    mutations.insert("QwQ", mutations_qux);
                    mutations.with_prefix("OwO")
                }
                _ => panic!("flat_flush can only be called on structs and maps"),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, U, _S: ?Sized, N> ::std::ops::Deref
    for FooObserver<'ob, S, T, U, _S, N>
    where
        T: Clone,
        U: ::morphix::Observe,
    {
        type Target = ::morphix::helper::Pointer<_S>;
        fn deref(&self) -> &Self::Target {
            &self.ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, U, _S: ?Sized, N> ::std::ops::DerefMut
    for FooObserver<'ob, S, T, U, _S, N>
    where
        T: Clone,
        U: ::morphix::Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.mutated = true;
            self.variant = FooObserverVariant::__Unknown;
            &mut self.ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, U, _S: ?Sized, N> ::morphix::helper::QuasiObserver
    for FooObserver<'ob, S, T, U, _S, N>
    where
        T: Clone,
        U: ::morphix::Observe,
        _S: ::morphix::helper::AsDeref<N>,
        N: ::morphix::helper::Unsigned,
    {
        type Head = _S;
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        type InnerDepth = N;
        fn invalidate(this: &mut Self) {
            this.mutated = true;
            this.variant = FooObserverVariant::__Unknown;
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, U, _S: ?Sized, N> ::morphix::observe::Observer
    for FooObserver<'ob, S, T, U, _S, N>
    where
        T: Clone,
        S: 'ob,
        Option<T>: 'ob,
        U: ::morphix::Observe,
        _S: ::morphix::helper::AsDerefMut<N, Target = Foo<S, T, U>>,
        N: ::morphix::helper::Unsigned,
    {
        fn observe(head: &mut _S) -> Self {
            let __value = head.as_deref_mut();
            Self {
                mutated: false,
                initial: FooObserverInitial::new(__value),
                variant: FooObserverVariant::observe(__value),
                ptr: ::morphix::helper::Pointer::new(head),
                phantom: ::std::marker::PhantomData,
            }
        }
        unsafe fn relocate(this: &mut Self, head: &mut _S) {
            let __value = head.as_deref_mut();
            unsafe { this.variant.relocate(__value) }
            ::morphix::helper::Pointer::set(this, head);
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, U, _S: ?Sized, N> ::morphix::observe::SerializeObserver
    for FooObserver<'ob, S, T, U, _S, N>
    where
        Foo<S, T, U>: ::morphix::helper::serde::Serialize + 'static,
        T: Clone,
        S: 'ob,
        Option<T>: 'ob,
        U: ::morphix::Observe,
        _S: ::morphix::helper::AsDeref<N, Target = Foo<S, T, U>>,
        N: ::morphix::helper::Unsigned,
        ::morphix::observe::DefaultObserver<
            'ob,
            U,
        >: ::morphix::observe::SerializeObserver,
    {
        unsafe fn flush(this: &mut Self) -> ::morphix::Mutations {
            let value = this.ptr.as_deref();
            let initial = this.initial;
            this.initial = FooObserverInitial::new(value);
            if !this.mutated {
                return this.variant.flush(value);
            }
            this.mutated = false;
            this.variant = FooObserverVariant::__Unknown;
            match (initial, value) {
                (FooObserverInitial::D, Foo::D)
                | (FooObserverInitial::E, Foo::E())
                | (FooObserverInitial::F, Foo::F {}) => ::morphix::Mutations::new(),
                _ => ::morphix::Mutations::replace(value),
            }
        }
        unsafe fn flat_flush(this: &mut Self) -> ::morphix::Mutations {
            let value = this.ptr.as_deref();
            let initial = this.initial;
            this.initial = FooObserverInitial::new(value);
            if !this.mutated {
                return this.variant.flat_flush(value);
            }
            this.mutated = false;
            this.variant = FooObserverVariant::__Unknown;
            match (initial, value) {
                (FooObserverInitial::D, Foo::D)
                | (FooObserverInitial::E, Foo::E())
                | (FooObserverInitial::F, Foo::F {}) => ::morphix::Mutations::new(),
                _ => ::morphix::Mutations::replace(value),
            }
        }
    }
    #[automatically_derived]
    impl<S, T, U> ::morphix::Observe for Foo<S, T, U>
    where
        Self: ::morphix::helper::serde::Serialize,
        T: Clone,
        U: ::morphix::Observe,
    {
        type Observer<'ob, _S, N> = FooObserver<'ob, S, T, U, _S, N>
        where
            Self: 'ob,
            U: 'ob,
            N: ::morphix::helper::Unsigned,
            _S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Qux {}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::general::Snapshot for Qux {
    type Snapshot = ();
    fn to_snapshot(&self) {}
    fn eq_snapshot(&self, snapshot: &()) -> bool {
        true
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::Observe for Qux {
    type Observer<'ob, S, N> = ::morphix::general::NoopObserver<'ob, S, N>
    where
        Self: 'ob,
        N: ::morphix::helper::Unsigned,
        S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
    type Spec = ::morphix::observe::SnapshotSpec;
}
