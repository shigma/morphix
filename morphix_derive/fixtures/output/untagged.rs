use ::std::fmt::Display;
#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(untagged, rename_all_fields = "UPPERCASE")]
pub enum Foo {
    A(u32),
    B(u32, u32),
    C { bar: String },
    D,
    E(),
    F {},
}
#[rustfmt::skip]
const _: () = {
    #[::std::prelude::v1::derive()]
    pub struct FooObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero> {
        __ptr: ::morphix::observe::ObserverPointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        __initial: ::std::mem::MaybeUninit<FooObserverInitial>,
        __variant: ::std::mem::MaybeUninit<FooObserverVariant<'ob>>,
    }
    #[derive(Clone, Copy)]
    pub enum FooObserverInitial {
        D,
        E,
        F,
        __Rest,
    }
    impl FooObserverInitial {
        fn new(value: &Foo) -> Self {
            match value {
                Foo::D => FooObserverInitial::D,
                Foo::E() => FooObserverInitial::E,
                Foo::F {} => FooObserverInitial::F,
                _ => FooObserverInitial::__Rest,
            }
        }
    }
    pub enum FooObserverVariant<'ob> {
        A(::morphix::observe::DefaultObserver<'ob, u32>),
        B(
            ::morphix::observe::DefaultObserver<'ob, u32>,
            ::morphix::observe::DefaultObserver<'ob, u32>,
        ),
        C { bar: ::morphix::observe::DefaultObserver<'ob, String> },
        D,
        E(),
        F {},
    }
    impl<'ob> FooObserverVariant<'ob> {
        fn observe(value: &'ob mut Foo) -> Self {
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
                Foo::D => Self::D,
                Foo::E() => Self::E(),
                Foo::F {} => Self::F {},
            }
        }
        unsafe fn refresh(&mut self, value: &mut Foo) {
            unsafe {
                match (self, value) {
                    (Self::A(u0), Foo::A(v0)) => {
                        ::morphix::observe::Observer::refresh(u0, v0)
                    }
                    (Self::B(u0, u1), Foo::B(v0, v1)) => {
                        ::morphix::observe::Observer::refresh(u0, v0);
                        ::morphix::observe::Observer::refresh(u1, v1)
                    }
                    (Self::C { bar: u0 }, Foo::C { bar: v0 }) => {
                        ::morphix::observe::Observer::refresh(u0, v0)
                    }
                    (Self::D, Foo::D) => {}
                    (Self::E(), Foo::E()) => {}
                    (Self::F {}, Foo::F {}) => {}
                    _ => panic!("inconsistent state for FooObserver"),
                }
            }
        }
        fn flush<A: ::morphix::Adapter>(
            &mut self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A::Value>>,
            A::Error,
        > {
            match self {
                Self::A(u0) => ::morphix::observe::SerializeObserver::flush::<A>(u0),
                Self::B(u0, u1) => {
                    let mut mutations = ::std::vec::Vec::with_capacity(2usize);
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(u0)? {
                        mutation.path.push("0".into());
                        mutations.push(mutation);
                    }
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(u1)? {
                        mutation.path.push("1".into());
                        mutations.push(mutation);
                    }
                    Ok(::morphix::Mutation::coalesce(mutations))
                }
                Self::C { bar } => {
                    match ::morphix::observe::SerializeObserver::flush::<A>(bar) {
                        Ok(Some(mut mutation)) => {
                            mutation.path.push("BAR".into());
                            Ok(Some(mutation))
                        }
                        result => result,
                    }
                }
                Self::D => Ok(None),
                Self::E() => Ok(None),
                Self::F {} => Ok(None),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::Deref for FooObserver<'ob, S, N> {
        type Target = ::morphix::observe::ObserverPointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::DerefMut for FooObserver<'ob, S, N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::helper::AsNormalized for FooObserver<'ob, S, N> {
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::Observer<'ob> for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type InnerDepth = N;
        fn uninit() -> Self {
            Self {
                __ptr: ::morphix::observe::ObserverPointer::uninit(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __initial: ::std::mem::MaybeUninit::uninit(),
                __variant: ::std::mem::MaybeUninit::uninit(),
            }
        }
        fn observe(value: &'ob mut S) -> Self {
            let __ptr = ::morphix::observe::ObserverPointer::new(value);
            let __value = value.as_deref_mut();
            Self {
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __initial: ::std::mem::MaybeUninit::new(
                    FooObserverInitial::new(__value),
                ),
                __variant: ::std::mem::MaybeUninit::new(
                    FooObserverVariant::observe(__value),
                ),
            }
        }
        unsafe fn refresh(this: &mut Self, value: &mut S) {
            ::morphix::observe::ObserverPointer::set(&this.__ptr, value);
            let __value = value.as_deref_mut();
            unsafe { this.__variant.assume_init_mut().refresh(__value) }
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::SerializeObserver<'ob>
    for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A::Value>>,
            A::Error,
        > {
            if !this.__mutated {
                return unsafe { this.__variant.assume_init_mut() }.flush::<A>();
            }
            let __value = this.__ptr.as_deref();
            let __initial = unsafe { this.__initial.assume_init() };
            this.__initial = ::std::mem::MaybeUninit::new(
                FooObserverInitial::new(__value),
            );
            match (__initial, __value) {
                (FooObserverInitial::D, Foo::D) => Ok(None),
                (FooObserverInitial::E, Foo::E()) => Ok(None),
                (FooObserverInitial::F, Foo::F {}) => Ok(None),
                _ => {
                    Ok(
                        Some(::morphix::Mutation {
                            path: ::morphix::Path::new(),
                            kind: ::morphix::MutationKind::Replace(
                                A::serialize_value(__value)?,
                            ),
                        }),
                    )
                }
            }
        }
    }
    #[automatically_derived]
    impl ::morphix::Observe for Foo {
        type Observer<'ob, S, N> = FooObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::fmt::Display for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        #[inline]
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            ::std::fmt::Display::fmt(self.as_deref(), f)
        }
    }
};
impl Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Foo::A(a) => write!(f, "Foo::A({})", a),
            Foo::B(a, b) => write!(f, "Foo::B({}, {})", a, b),
            Foo::C { bar } => write!(f, "Foo::C {{ bar: {} }}", bar),
            Foo::D => write!(f, "Foo::D"),
            Foo::E() => write!(f, "Foo::E()"),
            Foo::F {} => write!(f, "Foo::F {{}}"),
        }
    }
}
