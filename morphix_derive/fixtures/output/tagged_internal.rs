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
        fn observe(value: &Foo<N>) -> Self {
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
        unsafe fn refresh(&mut self, value: &Foo<N>) {
            unsafe {
                match (self, value) {
                    (Self::A(u0), Foo::A(v0)) => {
                        ::morphix::observe::Observer::refresh(u0, v0);
                    }
                    (Self::C { bar: u0, qux: u1 }, Foo::C { bar: v0, qux: v1 }) => {
                        ::morphix::observe::Observer::refresh(u0, v0);
                        ::morphix::observe::Observer::refresh(u1, v1);
                    }
                    (Self::__None, _) => {}
                    _ => panic!("inconsistent state for FooObserver"),
                }
            }
        }
        fn flush<A: ::morphix::Adapter>(
            &mut self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            match self {
                Self::A(u0) => {
                    let mutations_0 = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(u0)?;
                    let mut mutations = ::morphix::Mutations::with_capacity(
                        mutations_0.len(),
                    );
                    mutations.extend(mutations_0);
                    Ok(mutations)
                }
                Self::C { bar, qux } => {
                    let mut mutations_bar = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(bar)?;
                    if !mutations_bar.is_empty()
                        && String::is_empty(::morphix::observe::Observer::as_inner(bar))
                    {
                        mutations_bar = ::morphix::MutationKind::Delete.into();
                    }
                    let mutations_qux = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(qux)?;
                    let mut mutations = ::morphix::Mutations::with_capacity(
                        mutations_bar.len() + mutations_qux.len(),
                    );
                    mutations.insert("bar", mutations_bar);
                    mutations.extend(mutations_qux);
                    Ok(mutations)
                }
                Self::__None => Ok(::morphix::Mutations::new()),
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
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        type InnerDepth = _N;
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::morphix::observe::Observer
    for FooObserver<'ob, N, S, _N>
    where
        S: ::morphix::helper::AsDerefMut<_N, Target = Foo<N>> + 'ob,
        _N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self {
                __ptr: ::morphix::helper::Pointer::uninit(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __variant: FooObserverVariant::__None,
            }
        }
        fn observe(value: &S) -> Self {
            let __ptr = ::morphix::helper::Pointer::new(value);
            let __value = value.as_deref();
            Self {
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __variant: FooObserverVariant::observe(__value),
            }
        }
        unsafe fn refresh(this: &mut Self, value: &S) {
            ::morphix::helper::Pointer::set(this, value);
            let __value = value.as_deref();
            unsafe { this.__variant.refresh(__value) }
        }
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::morphix::observe::SerializeObserver
    for FooObserver<'ob, N, S, _N>
    where
        Foo<N>: ::serde::Serialize,
        S: ::morphix::helper::AsDerefMut<_N, Target = Foo<N>> + 'ob,
        _N: ::morphix::helper::Unsigned,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            if !this.__mutated {
                return this.__variant.flush::<A>();
            }
            this.__mutated = false;
            this.__variant = FooObserverVariant::__None;
            Ok(
                ::morphix::MutationKind::Replace(A::serialize_value(this.as_deref())?)
                    .into(),
            )
        }
    }
    #[automatically_derived]
    impl<const N: usize> ::morphix::Observe for Foo<N>
    where
        Self: ::serde::Serialize,
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
    #[inline]
    fn to_snapshot(&self) {}
    #[inline]
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
