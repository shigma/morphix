macro_rules! spec_impl_observe {
    ($helper:ident, $ty_self:ty, $ty_t:ty, $default:ident $(, const $arg:ident: $arg_ty:ty)* $(,)?) => {
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
                $crate::observe::Observer<'ob, Head = S, InnerDepth = D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = $ty_self> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::DefaultSpec> for T
        where
            T: Observe<Spec = $crate::observe::DefaultSpec>,
        {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>
                = $default<'ob, T::Observer<'ob, T, Zero>, S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = $ty_self> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::SnapshotSpec> for T
        where
            T: Clone + PartialEq + Observe<Spec = $crate::observe::SnapshotSpec>,
        {
            type Observer<'ob, S, D $(, const $arg: $arg_ty)*>
                = $crate::observe::SnapshotObserver<'ob, S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = $ty_self> + ?Sized + 'ob;
        }
    };
}

macro_rules! spec_impl_ref_observe {
    ($helper:ident, $ty_self:ty, $ty_t:ty $(, const $arg:ident: $arg_ty:ty)* $(,)?) => {
        impl<T $(, const $arg: $arg_ty)*> $crate::observe::RefObserve for $ty_t
        where
            T: $crate::observe::RefObserve + $helper<T::Spec>,
        {
            type Observer<'ob, S, D, E>
                = <T as $helper<T::Spec>>::Observer<'ob, S, D, E $(, $arg)*>
            where
                Self: 'ob,
                D: Unsigned,
                E: Unsigned,
                S: $crate::helper::AsDeref<D> + ?Sized + 'ob,
                S::Target: $crate::helper::AsDeref<E, Target = Self>;

            type Spec = T::Spec;
        }

        pub trait $helper<Spec> {
            type Observer<'ob, S, D, E $(, const $arg: $arg_ty)*>:
                $crate::observe::Observer<'ob, Head = S, InnerDepth = D>
            where
                Self: 'ob,
                D: Unsigned,
                E: Unsigned,
                S: $crate::helper::AsDeref<D, Target: $crate::helper::AsDeref<E, Target = $ty_self>> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::DefaultSpec> for T
        where
            T: Observe<Spec = $crate::observe::DefaultSpec>,
        {
            type Observer<'ob, S, D, E $(, const $arg: $arg_ty)*>
                = $crate::observe::RefObserver<'ob, S, D, E>
            where
                Self: 'ob,
                D: Unsigned,
                E: Unsigned,
                S: $crate::helper::AsDeref<D, Target: $crate::helper::AsDeref<E, Target = $ty_self>> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::SnapshotSpec> for T
        where
            T: Clone + PartialEq + Observe<Spec = $crate::observe::SnapshotSpec>,
        {
            type Observer<'ob, S, D, E $(, const $arg: $arg_ty)*>
                = $crate::observe::SnapshotObserver<'ob, S, D, E>
            where
                Self: 'ob,
                D: Unsigned,
                E: Unsigned,
                S: $crate::helper::AsDeref<D, Target: $crate::helper::AsDeref<E, Target = $ty_self>> + ?Sized + 'ob;
        }
    };
}

macro_rules! default_impl_ref_observe {
    ($(impl $([$($gen:tt)*])? RefObserve for $ty:ty $(where { $($where:tt)+ })?;)*) => {
        $(
            impl <$($($gen)*)?> $crate::observe::RefObserve for $ty {
                type Observer<'ob, S, D, E>
                    = $crate::observe::RefObserver<'ob, S, D, E>
                where
                    Self: 'ob,
                    D: Unsigned,
                    E: Unsigned,
                    S: $crate::helper::AsDeref<D> + ?Sized + 'ob,
                    S::Target: $crate::helper::AsDeref<E, Target = Self>;

                type Spec = $crate::observe::DefaultSpec;
            }
        )*
    };
}

macro_rules! untracked_methods {
    ($type:ident => $(
        // Wrap {} around where clauses for easier parsing
        pub fn $name:ident $(<$($gen:tt),*>)? (&mut self $(, $arg:ident: $arg_ty:ty)*) $(-> $ret:ty)? $(where { $($where:tt)+ })?;
    )*) => {
        $(
            #[doc = concat!(" See [`", stringify!($type), "::", stringify!($name), "`].")]
            #[inline]
            pub fn $name $(<$($gen),*>)? (&mut self $(, $arg: $arg_ty)*) $(-> $ret)? $(where $($where)+)? {
                Observer::as_inner(self).$name($($arg),*)
            }
        )*
    };
}

pub(crate) use {default_impl_ref_observe, spec_impl_observe, spec_impl_ref_observe, untracked_methods};
