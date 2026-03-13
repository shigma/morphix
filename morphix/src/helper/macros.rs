macro_rules! spec_impl_observe {
    ($(#[$($tt:tt)*])* $helper:ident, $ty_self:ty, $ty_t:ty, $default:ident $(, const $arg:ident: $arg_ty:ty)* $(,)?) => {
        $(#[$($tt)*])*
        impl<T $(, const $arg: $arg_ty)*> $crate::observe::Observe for $ty_t
        where
            T: $crate::observe::Observe + $helper<T::Spec>,
        {
            type Observer<'ob, S, D>
                = <T as $helper<T::Spec>>::Observer<'ob, S, D $(, $arg)*>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

            type Spec = T::Spec;
        }

        pub trait $helper<Spec> {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>:
                $crate::observe::Observer<Head = S, InnerDepth = D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = $ty_self> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::DefaultSpec> for T
        where
            T: $crate::observe::Observe<Spec = $crate::observe::DefaultSpec>,
        {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>
                = $default<T::Observer<'ob, T, Zero>, S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = $ty_self> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::SnapshotSpec> for T
        where
            T: $crate::builtin::Snapshot + $crate::observe::Observe<Spec = $crate::observe::SnapshotSpec>,
        {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>
                = $crate::builtin::SnapshotObserver<'ob, S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = $ty_self> + ?Sized + 'ob;
        }
    };
}

macro_rules! spec_impl_observe_from_ref {
    ($(#[$($tt:tt)*])* $helper:ident, $ty_self:ty, $ty_t:ty, $default:ident $(, const $arg:ident: $arg_ty:ty)* $(,)?) => {
        $(#[$($tt)*])*
        impl<T $(, const $arg: $arg_ty)*> $crate::observe::Observe for $ty_t
        where
            T: $crate::observe::RefObserve + $helper<T::Spec>,
        {
            type Observer<'ob, S, D>
                = <T as $helper<T::Spec>>::Observer<'ob, S, D $(, $arg)*>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

            type Spec = T::Spec;
        }

        pub trait $helper<Spec> {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>:
                $crate::observe::Observer<Head = S, InnerDepth = D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = $ty_self> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::DefaultSpec> for T
        where
            T: $crate::observe::RefObserve<Spec = $crate::observe::DefaultSpec>,
        {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>
                = $default<T::Observer<'ob, T, Zero>, S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = $ty_self> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::SnapshotSpec> for T
        where
            T: $crate::builtin::Snapshot + $crate::observe::RefObserve<Spec = $crate::observe::SnapshotSpec>,
        {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>
                = $crate::builtin::SnapshotObserver<'ob, S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = $ty_self> + ?Sized + 'ob;
        }
    };
}

macro_rules! spec_impl_ref_observe {
    ($(#[$($tt:tt)*])* $helper:ident, $ty_self:ty, $ty_t:ty, $default:ident $(, const $arg:ident: $arg_ty:ty)* $(,)?) => {
        $(#[$($tt)*])*
        impl<T $(, const $arg: $arg_ty)*> $crate::observe::RefObserve for $ty_t
        where
            T: $crate::observe::RefObserve + $helper<T::Spec>,
        {
            type Observer<'ob, S, D>
                = <T as $helper<T::Spec>>::Observer<'ob, S, D $(, $arg)*>
            where
                Self: 'ob,
                D: Unsigned,
                S: $crate::helper::AsDeref<D, Target = Self> + ?Sized + 'ob;

            type Spec = T::Spec;
        }

        pub trait $helper<Spec> {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>:
                $crate::observe::RefObserver<Head = S, InnerDepth = D>
            where
                Self: 'ob,
                D: Unsigned,
                S: $crate::helper::AsDeref<D, Target = $ty_self> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::DefaultSpec> for T
        where
            T: $crate::observe::RefObserve<Spec = $crate::observe::DefaultSpec>,
        {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>
                = $default<$($arg,)* T::Observer<'ob, T, Zero>, S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: $crate::helper::AsDeref<D, Target = $ty_self> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::SnapshotSpec> for T
        where
            T: $crate::builtin::Snapshot + $crate::observe::RefObserve<Spec = $crate::observe::SnapshotSpec>,
        {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>
                = $crate::builtin::SnapshotObserver<'ob, S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: $crate::helper::AsDeref<D, Target = $ty_self> + ?Sized + 'ob;
        }
    };
}

macro_rules! default_impl_ref_observe {
    ($(impl $([$($gen:tt)*])? RefObserve for $ty:ty $(where { $($where:tt)+ })?;)*) => {
        $(
            impl <$($($gen)*)?> $crate::observe::RefObserve for $ty {
                type Observer<'ob, S, D>
                    = $crate::builtin::PointerObserver<'ob, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: $crate::helper::AsDeref<D, Target = Self> + ?Sized + 'ob;

                type Spec = $crate::observe::DefaultSpec;
            }
        )*
    };
}

macro_rules! shallow_observer {
    (impl $([$($gen:tt)*])? $ob:ident for $ty:ty;) => {
        #[doc = concat!("Observer implementation for [`", stringify!($ty), "`].")]
        pub struct $ob<'ob, S: ?Sized, D = $crate::helper::Zero> {
            ptr: $crate::helper::Pointer<S>,
            mutated: bool,
            phantom: ::std::marker::PhantomData<&'ob mut D>,
        }

        impl<'ob, S: ?Sized, D> ::std::ops::Deref for $ob<'ob, S, D> {
            type Target = $crate::helper::Pointer<S>;

            fn deref(&self) -> &Self::Target {
                &self.ptr
            }
        }

        impl<'ob, S: ?Sized, D> ::std::ops::DerefMut for $ob<'ob, S, D> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.mutated = true;
                $crate::helper::Pointer::invalidate(&mut self.ptr);
                &mut self.ptr
            }
        }

        impl<'ob, S: ?Sized, D> QuasiObserver for $ob<'ob, S, D>
        where
            D: Unsigned,
            S: $crate::helper::AsDeref<D>,
        {
            type Head = S;
            type OuterDepth = $crate::helper::Succ<$crate::helper::Zero>;
            type InnerDepth = D;

            fn invalidate(this: &mut Self) {
                this.mutated = true;
            }
        }

        impl<'ob, S: ?Sized, D> $crate::observe::Observer for $ob<'ob, S, D>
        where
            D: Unsigned,
            S: AsDerefMut<D>,
        {
            fn uninit() -> Self {
                Self {
                    ptr: $crate::helper::Pointer::uninit(),
                    mutated: false,
                    phantom: ::std::marker::PhantomData,
                }
            }

            unsafe fn refresh(this: &mut Self, head: &mut Self::Head) {
                $crate::helper::Pointer::set(this, head);
            }

            fn observe(head: &mut Self::Head) -> Self {
                Self {
                    ptr: $crate::helper::Pointer::new(head),
                    mutated: false,
                    phantom: ::std::marker::PhantomData,
                }
            }
        }

        impl<'ob, S: ?Sized, D> $crate::observe::SerializeObserver for $ob<'ob, S, D>
        where
            D: Unsigned,
            S: $crate::helper::AsDeref<D, Target: ::serde::Serialize + 'static>,
        {
            unsafe fn flush(this: &mut Self) -> $crate::mutation::Mutations {
                if ::std::mem::take(&mut this.mutated) {
                    $crate::mutation::Mutations::replace((*this.ptr).as_deref())
                } else {
                    $crate::mutation::Mutations::new()
                }
            }
        }

        impl $(<$($gen)*>)? Observe for $ty {
            type Observer<'ob, S, D>
                = $ob<'ob, S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

            type Spec = DefaultSpec;
        }
    };
}

macro_rules! delegate_methods {
    ($($delegate:ident()).+ as $type:ident => $($tokens:tt)*) => {
        delegate_methods!(@fn ($($delegate()).+) as $type => [] $($tokens)*);
    };

    (@fn ($($delegate:tt)*) as $type:ident => []) => {};

    (@fn ($($delegate:tt)*) as $type:ident => [] $(#[$meta:meta])* pub fn $name:ident $($rest:tt)*) => {
        delegate_methods!(@fn ($($delegate)*) as $type => [$(#[$meta])* pub fn $name] $($rest)*);
    };

    (@fn ($($delegate:tt)*) as $type:ident => [] $(#[$meta:meta])* pub unsafe fn $name:ident $($rest:tt)*) => {
        delegate_methods!(@fn ($($delegate)*) as $type => [$(#[$meta])* pub unsafe fn $name] $($rest)*);
    };

    (@fn ($($delegate:tt)*) as $type:ident => [$($head:tt)*] ($($arg:tt)*) $(-> $ty:ty)?; $($rest:tt)*) => {
        delegate_methods!(@impl ($($delegate)*) as $type => $($head)* [] ($($arg)*) $(-> $ty)?);
        delegate_methods!(@fn ($($delegate)*) as $type => [] $($rest)*);
    };

    (@fn ($($delegate:tt)*) as $type:ident => [$($head:tt)*] ($($arg:tt)*) $(-> $ty:ty)? where $($rest:tt)*) => {
        delegate_methods!(@where ($($delegate)*) as $type => [$($head)*] [] (($($arg)*) $(-> $ty)? where) $($rest)*);
    };

    (@fn ($($delegate:tt)*) as $type:ident => [$($head:tt)*] < $($rest:tt)*) => {
        delegate_methods!(@gen ($($delegate)*) as $type => [$($head)*] [] $($rest)*);
    };

    (@gen ($($delegate:tt)*) as $type:ident => [$($head:tt)*] [$($gen:tt)*] > ($($arg:tt)*) $(-> $ty:ty)?; $($rest:tt)*) => {
        delegate_methods!(@impl ($($delegate)*) as $type => $($head)* [$($gen)*] ($($arg)*) $(-> $ty)?);
        delegate_methods!(@fn ($($delegate)*) as $type => [] $($rest)*);
    };

    (@gen ($($delegate:tt)*) as $type:ident => [$($head:tt)*] [$($gen:tt)*] > ($($arg:tt)*) $(-> $ty:ty)? where $($rest:tt)*) => {
        delegate_methods!(@where ($($delegate)*) as $type => [$($head)*] [$($gen)*] (($($arg)*) $(-> $ty)? where) $($rest)*);
    };

    (@gen ($($delegate:tt)*) as $type:ident => [$($head:tt)*] [$($gen:tt)*] >> ($($arg:tt)*) $(-> $ty:ty)?; $($rest:tt)*) => {
        delegate_methods!(@impl ($($delegate)*) as $type => $($head)* [$($gen)* >] ($($arg)*) $(-> $ty)?);
        delegate_methods!(@fn ($($delegate)*) as $type => [] $($rest)*);
    };

    (@gen ($($delegate:tt)*) as $type:ident => [$($head:tt)*] [$($gen:tt)*] >> ($($arg:tt)*) $(-> $ty:ty)? where $($rest:tt)*) => {
        delegate_methods!(@where ($($delegate)*) as $type => [$($head)*] [$($gen)* >] (($($arg)*) $(-> $ty)? where) $($rest)*);
    };

    (@gen ($($delegate:tt)*) as $type:ident => [$($head:tt)*] [$($gen:tt)*] $tt:tt $($rest:tt)*) => {
        delegate_methods!(@gen ($($delegate)*) as $type => [$($head)*] [$($gen)* $tt] $($rest)*);
    };

    (@where ($($delegate:tt)*) as $type:ident => [$($head:tt)*] [$($gen:tt)*] ($($tail:tt)*); $($rest:tt)*) => {
        delegate_methods!(@impl ($($delegate)*) as $type => $($head)* [$($gen)*] $($tail)*);
        delegate_methods!(@fn ($($delegate)*) as $type => [] $($rest)*);
    };

    (@where ($($delegate:tt)*) as $type:ident => [$($head:tt)*] [$($gen:tt)*] ($($tail:tt)*) $tt:tt $($rest:tt)*) => {
        delegate_methods!(@where ($($delegate)*) as $type => [$($head)*] [$($gen)*] ($($tail)* $tt) $($rest)*);
    };

    (@impl ($($delegate:tt)*) as $type:ident =>
        $(#[$meta:meta])*
        pub fn $name:ident [$($gen:tt)*] (&mut self $(, $arg:ident : $arg_ty:ty)*) $($rest:tt)*
    ) => {
        $(#[$meta])*
        #[doc = ""]
        #[doc = concat!(" See [`", stringify!($type), "::", stringify!($name), "`].")]
        pub fn $name <$($gen)*> (&mut self $(, $arg: $arg_ty)*) $($rest)* {
            self.$($delegate)*.$name($($arg),*)
        }
    };

    (@impl ($($delegate:tt)*) as $type:ident =>
        $(#[$meta:meta])*
        pub unsafe fn $name:ident [$($gen:tt)*] (&mut self $(, $arg:ident : $arg_ty:ty)*) $($rest:tt)*
    ) => {
        $(#[$meta])*
        #[doc = ""]
        #[doc = concat!(" See [`", stringify!($type), "::", stringify!($name), "`].")]
        #[doc = ""]
        #[doc = "## Safety"]
        #[doc = ""]
        #[doc = concat!(" See [`", stringify!($type), "::", stringify!($name), "`] for safety requirements.")]
        pub unsafe fn $name <$($gen)*> (&mut self $(, $arg: $arg_ty)*) $($rest)* {
            unsafe { self.$($delegate)*.$name($($arg),*) }
        }
    };
}

pub(crate) use {
    default_impl_ref_observe, delegate_methods, shallow_observer, spec_impl_observe, spec_impl_observe_from_ref,
    spec_impl_ref_observe,
};
