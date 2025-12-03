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

        #[cfg(feature = "hash")]
        const _: () = {
            impl<T> $helper<$crate::observe::HashSpec> for T
            where
                T: ::std::hash::Hash + Observe<Spec = $crate::observe::HashSpec>,
            {
                type Observer<'ob, S, D $(, const $arg: $arg_ty)*>
                    = $crate::observe::HashObserver<'ob, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = $ty_self> + ?Sized + 'ob;
            }
        };

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
            type Observer<'a, 'ob, S, D>
                = <T as $helper<T::Spec>>::Observer<'a, 'ob, S, D $(, $arg)*>
            where
                Self: 'a + 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = &'a Self> + ?Sized + 'ob;

            type Spec = T::Spec;
        }

        pub trait $helper<Spec> {
            type Observer<'a, 'ob, S, D $(, const $arg: $arg_ty)*>:
                $crate::observe::Observer<'ob, Head = S, InnerDepth = D>
            where
                Self: 'a + 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = &'a $ty_self> + ?Sized + 'ob;
        }

        impl<T> $helper<$crate::observe::DefaultSpec> for T
        where
            T: Observe<Spec = $crate::observe::DefaultSpec>,
        {
            type Observer<'a, 'ob, S, D $(, const $arg: $arg_ty)*>
                = $crate::observe::RefObserver<'a, 'ob, S, D>
            where
                Self: 'a + 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = &'a $ty_self> + ?Sized + 'ob;
        }

        #[cfg(feature = "hash")]
        const _: () = {
            use std::hash::Hash;

            use crate::observe::{HashObserver, HashSpec};

            impl<T> $helper<HashSpec> for T
            where
                T: Hash + Observe<Spec = HashSpec>,
            {
                type Observer<'a, 'ob, S, D $(, const $arg: $arg_ty)*>
                    = HashObserver<'ob, S, D>
                where
                    Self: 'a + 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = &'a $ty_self> + ?Sized + 'ob;
            }
        };

        impl<T> $helper<$crate::observe::SnapshotSpec> for T
        where
            T: Clone + PartialEq + Observe<Spec = $crate::observe::SnapshotSpec>,
        {
            type Observer<'a, 'ob, S, D $(, const $arg: $arg_ty)*>
                = $crate::observe::SnapshotObserver<'ob, S, D>
            where
                Self: 'a + 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = &'a $ty_self> + ?Sized + 'ob;
        }
    };
}

pub(crate) use {spec_impl_observe, spec_impl_ref_observe};
