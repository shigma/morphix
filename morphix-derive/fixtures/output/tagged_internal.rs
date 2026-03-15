#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum Foo<const N: usize> {
    #[serde(serialize_with = "<[_]>::serialize")]
    A([u32; N]),
    C {
        #[serde(skip_serializing_if = "String::is_empty")]
        bar: String,
        #[serde(flatten)]
        qux: Qux,
    },
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<
        'ob,
        const N: usize,
        S: ?Sized,
        _N = ::morphix::helper::Zero,
    > {
        __ptr: ::morphix::helper::Pointer<S>,
        __mutated: bool,
        __variant: FooObserverVariant<'ob, N>,
        __phantom: ::std::marker::PhantomData<&'ob mut _N>,
    }
    pub enum FooObserverVariant<'ob, const N: usize> {
        A(::morphix::observe::DefaultObserver<'ob, [u32; N]>),
        C {
            bar: ::morphix::observe::DefaultObserver<'ob, String>,
            qux: ::morphix::observe::DefaultObserver<'ob, Qux>,
        },
        __None,
    }
    impl<'ob, const N: usize> FooObserverVariant<'ob, N> {
        fn observe(value: &mut Foo<N>) -> Self {
            match value {
                Foo::A(v0) => Self::A(::morphix::observe::Observer::observe(v0)),
                Foo::C { bar, qux } => {
                    Self::C {
                        bar: ::morphix::observe::Observer::observe(bar),
                        qux: ::morphix::observe::Observer::observe(qux),
                    }
                }
            }
        }
        unsafe fn relocate(&mut self, value: &mut Foo<N>) {
            unsafe {
                match (self, value) {
                    (Self::A(u0), Foo::A(v0)) => {
                        ::morphix::observe::Observer::relocate(u0, v0);
                    }
                    (Self::C { bar: u0, qux: u1 }, Foo::C { bar: v0, qux: v1 }) => {
                        ::morphix::observe::Observer::relocate(u0, v0);
                        ::morphix::observe::Observer::relocate(u1, v1);
                    }
                    (Self::__None, _) => {}
                    _ => panic!("inconsistent state for FooObserver"),
                }
            }
        }
        fn flush(&mut self) -> ::morphix::Mutations {
            match self {
                Self::A(u0) => {
                    let mutations_0 = unsafe {
                        ::morphix::observe::SerializeObserver::flush(u0)
                    };
                    let mut mutations = ::morphix::Mutations::with_capacity(
                        mutations_0.len(),
                    );
                    mutations.extend(mutations_0);
                    mutations
                }
                Self::C { bar, qux } => {
                    let mut mutations_bar = unsafe {
                        ::morphix::observe::SerializeObserver::flush(bar)
                    };
                    if !mutations_bar.is_empty()
                        && String::is_empty(
                            ::morphix::helper::QuasiObserver::untracked_ref(bar),
                        )
                    {
                        mutations_bar = ::morphix::MutationKind::Delete.into();
                    }
                    let mutations_qux = unsafe {
                        ::morphix::observe::SerializeObserver::flush(qux)
                    };
                    let mut mutations = ::morphix::Mutations::with_capacity(
                        mutations_bar.len() + mutations_qux.len(),
                    );
                    mutations.insert("bar", mutations_bar);
                    mutations.extend(mutations_qux);
                    mutations
                }
                Self::__None => ::morphix::Mutations::new(),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::std::ops::Deref
    for FooObserver<'ob, N, S, _N> {
        type Target = ::morphix::helper::Pointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::std::ops::DerefMut
    for FooObserver<'ob, N, S, _N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.__mutated = true;
            self.__variant = FooObserverVariant::__None;
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::morphix::helper::QuasiObserver
    for FooObserver<'ob, N, S, _N>
    where
        S: ::morphix::helper::AsDeref<_N>,
        _N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        type InnerDepth = _N;
        fn invalidate(this: &mut Self) {
            this.__mutated = true;
            this.__variant = FooObserverVariant::__None;
        }
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::morphix::observe::Observer
    for FooObserver<'ob, N, S, _N>
    where
        S: ::morphix::helper::AsDerefMut<_N, Target = Foo<N>>,
        _N: ::morphix::helper::Unsigned,
    {
        fn observe(head: &mut S) -> Self {
            let __value = head.as_deref_mut();
            Self {
                __mutated: false,
                __variant: FooObserverVariant::observe(__value),
                __ptr: ::morphix::helper::Pointer::new(head),
                __phantom: ::std::marker::PhantomData,
            }
        }
        unsafe fn relocate(this: &mut Self, head: &mut S) {
            let __value = head.as_deref_mut();
            unsafe { this.__variant.relocate(__value) }
            ::morphix::helper::Pointer::set(this, head);
        }
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::morphix::observe::SerializeObserver
    for FooObserver<'ob, N, S, _N>
    where
        Foo<N>: ::morphix::helper::serde::Serialize + 'static,
        S: ::morphix::helper::AsDeref<_N, Target = Foo<N>>,
        _N: ::morphix::helper::Unsigned,
    {
        unsafe fn flush(this: &mut Self) -> ::morphix::Mutations {
            if !this.__mutated {
                return this.__variant.flush();
            }
            this.__mutated = false;
            this.__variant = FooObserverVariant::__None;
            ::morphix::Mutations::replace(this.as_deref())
        }
    }
    #[automatically_derived]
    impl<const N: usize> ::morphix::Observe for Foo<N>
    where
        Self: ::morphix::helper::serde::Serialize,
    {
        type Observer<'ob, S, _N> = FooObserver<'ob, N, S, _N>
        where
            Self: 'ob,
            _N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<_N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Qux {}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::builtin::Snapshot for Qux {
    type Snapshot = ();
    fn to_snapshot(&self) {}
    fn eq_snapshot(&self, snapshot: &()) -> bool {
        true
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::Observe for Qux {
    type Observer<'ob, S, N> = ::morphix::builtin::NoopObserver<'ob, S, N>
    where
        Self: 'ob,
        N: ::morphix::helper::Unsigned,
        S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
    type Spec = ::morphix::observe::SnapshotSpec;
}
