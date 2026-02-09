use ::std::fmt::Display;
#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Foo {
    r#a: i32,
    #[serde(rename = "bar")]
    b: String,
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero> {
        r#a: ::morphix::observe::DefaultObserver<'ob, i32>,
        b: ::morphix::observe::DefaultObserver<'ob, String>,
        __ptr: ::morphix::helper::Pointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::Deref for FooObserver<'ob, S, N> {
        type Target = ::morphix::helper::Pointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::DerefMut for FooObserver<'ob, S, N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.__mutated = true;
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
                r#a: ::morphix::observe::Observer::uninit(),
                b: ::morphix::observe::Observer::uninit(),
                __ptr: ::morphix::helper::Pointer::uninit(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
            }
        }
        fn observe(value: &'ob mut S) -> Self {
            let __ptr = ::morphix::helper::Pointer::new(value);
            let __value = value.as_deref_mut();
            Self {
                r#a: ::morphix::observe::Observer::observe(&mut __value.r#a),
                b: ::morphix::observe::Observer::observe(&mut __value.b),
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
            }
        }
        unsafe fn refresh(this: &mut Self, value: &mut S) {
            ::morphix::helper::Pointer::set(this, value);
            let __value = value.as_deref_mut();
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.r#a, &mut __value.r#a);
                ::morphix::observe::Observer::refresh(&mut this.b, &mut __value.b);
            }
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
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            if this.__mutated {
                this.__mutated = false;
                return Ok(
                    ::morphix::MutationKind::Replace(
                            A::serialize_value(this.as_deref())?,
                        )
                        .into(),
                );
            }
            let mutations_a = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.r#a)?;
            let mutations_b = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.b)?;
            let mut mutations = ::morphix::Mutations::with_capacity(
                mutations_a.len() + mutations_b.len(),
            );
            mutations.insert("A", mutations_a);
            mutations.insert("bar", mutations_b);
            Ok(mutations)
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
    impl<'ob, S: ?Sized, N> ::std::fmt::Debug for FooObserver<'ob, S, N> {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("FooObserver")
                .field("a", &self.r#a)
                .field("b", &self.b)
                .finish()
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::fmt::Display for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        #[inline]
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let head = &**::morphix::helper::AsNormalized::as_normalized_ref(self);
            let value = ::morphix::helper::AsDeref::<N>::as_deref(head);
            ::std::fmt::Display::fmt(value, f)
        }
    }
};
impl Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Foo {{ a: {}, b: {} }}", self.a, self.b)
    }
}
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Bar(i32);
#[rustfmt::skip]
pub struct BarObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero>(
    ::morphix::observe::DefaultObserver<'ob, i32>,
    ::morphix::helper::Pointer<S>,
    bool,
    ::std::marker::PhantomData<&'ob mut N>,
);
#[rustfmt::skip]
#[automatically_derived]
impl<'ob, S: ?Sized, N> ::std::ops::Deref for BarObserver<'ob, S, N> {
    type Target = ::morphix::helper::Pointer<S>;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl<'ob, S: ?Sized, N> ::std::ops::DerefMut for BarObserver<'ob, S, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.2 = true;
        &mut self.1
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl<'ob, S: ?Sized, N> ::morphix::helper::AsNormalized for BarObserver<'ob, S, N> {
    type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
}
#[rustfmt::skip]
#[automatically_derived]
impl<'ob, S: ?Sized, N> ::morphix::observe::Observer<'ob> for BarObserver<'ob, S, N>
where
    S: ::morphix::helper::AsDerefMut<N, Target = Bar> + 'ob,
    N: ::morphix::helper::Unsigned,
{
    type Head = S;
    type InnerDepth = N;
    fn uninit() -> Self {
        Self(
            ::morphix::observe::Observer::uninit(),
            ::morphix::helper::Pointer::uninit(),
            false,
            ::std::marker::PhantomData,
        )
    }
    fn observe(value: &'ob mut S) -> Self {
        let __ptr = ::morphix::helper::Pointer::new(value);
        let __value = value.as_deref_mut();
        Self(
            ::morphix::observe::Observer::observe(&mut __value.0),
            __ptr,
            false,
            ::std::marker::PhantomData,
        )
    }
    unsafe fn refresh(this: &mut Self, value: &mut S) {
        ::morphix::helper::Pointer::set(this, value);
        let __value = value.as_deref_mut();
        unsafe {
            ::morphix::observe::Observer::refresh(&mut this.0, &mut __value.0);
        }
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl<'ob, S: ?Sized, N> ::morphix::observe::SerializeObserver<'ob>
for BarObserver<'ob, S, N>
where
    S: ::morphix::helper::AsDerefMut<N, Target = Bar> + 'ob,
    N: ::morphix::helper::Unsigned,
{
    unsafe fn flush_unchecked<A: ::morphix::Adapter>(
        this: &mut Self,
    ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
        if this.2 {
            this.2 = false;
            return Ok(
                ::morphix::MutationKind::Replace(A::serialize_value(this.as_deref())?)
                    .into(),
            );
        }
        let mutations_0 = ::morphix::observe::SerializeObserver::flush::<
            A,
        >(&mut this.0)?;
        let mut mutations = ::morphix::Mutations::with_capacity(mutations_0.len());
        mutations.extend(mutations_0);
        Ok(mutations)
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::Observe for Bar {
    type Observer<'ob, S, N> = BarObserver<'ob, S, N>
    where
        Self: 'ob,
        N: ::morphix::helper::Unsigned,
        S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
    type Spec = ::morphix::observe::DefaultSpec;
}
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Baz(i32, String);
#[rustfmt::skip]
const _: () = {
    pub struct BazObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero>(
        ::morphix::observe::DefaultObserver<'ob, i32>,
        ::morphix::observe::DefaultObserver<'ob, String>,
        ::morphix::helper::Pointer<S>,
        bool,
        ::std::marker::PhantomData<&'ob mut N>,
    );
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::Deref for BazObserver<'ob, S, N> {
        type Target = ::morphix::helper::Pointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.2
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::DerefMut for BazObserver<'ob, S, N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.3 = true;
            &mut self.2
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::helper::AsNormalized for BazObserver<'ob, S, N> {
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::Observer<'ob> for BazObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Baz> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type InnerDepth = N;
        fn uninit() -> Self {
            Self(
                ::morphix::observe::Observer::uninit(),
                ::morphix::observe::Observer::uninit(),
                ::morphix::helper::Pointer::uninit(),
                false,
                ::std::marker::PhantomData,
            )
        }
        fn observe(value: &'ob mut S) -> Self {
            let __ptr = ::morphix::helper::Pointer::new(value);
            let __value = value.as_deref_mut();
            Self(
                ::morphix::observe::Observer::observe(&mut __value.0),
                ::morphix::observe::Observer::observe(&mut __value.1),
                __ptr,
                false,
                ::std::marker::PhantomData,
            )
        }
        unsafe fn refresh(this: &mut Self, value: &mut S) {
            ::morphix::helper::Pointer::set(this, value);
            let __value = value.as_deref_mut();
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.0, &mut __value.0);
                ::morphix::observe::Observer::refresh(&mut this.1, &mut __value.1);
            }
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::SerializeObserver<'ob>
    for BazObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Baz> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            if this.3 {
                this.3 = false;
                return Ok(
                    ::morphix::MutationKind::Replace(
                            A::serialize_value(this.as_deref())?,
                        )
                        .into(),
                );
            }
            let mutations_0 = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.0)?;
            let mutations_1 = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.1)?;
            let mut mutations = ::morphix::Mutations::with_capacity(
                mutations_0.len() + mutations_1.len(),
            );
            mutations.insert(0usize, mutations_0);
            mutations.insert(1usize, mutations_1);
            Ok(mutations)
        }
    }
    #[automatically_derived]
    impl ::morphix::Observe for Baz {
        type Observer<'ob, S, N> = BazObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::fmt::Debug for BazObserver<'ob, S, N> {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_tuple("BazObserver").field(&self.0).field(&self.1).finish()
        }
    }
};
