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

macro_rules! delegate_methods {
    ($($delegate:ident()).+ as $type:ident => $($tokens:tt)*) => {
        delegate_methods!(@item ($($delegate()).+) as $type => () $($tokens)*);
    };

    (@item ($($delegate:tt)*) as $type:ident => ()) => {};

    (@item ($($delegate:tt)*) as $type:ident => () $(#[$meta:meta])* pub fn $name:ident $($rest:tt)*) => {
        delegate_methods!(@item ($($delegate)*) as $type => ($(#[$meta])* pub fn $name) $($rest)*);
    };

    (@item ($($delegate:tt)*) as $type:ident => () $(#[$meta:meta])* pub unsafe fn $name:ident $($rest:tt)*) => {
        delegate_methods!(@item ($($delegate)*) as $type => ($(#[$meta])* pub unsafe fn $name) $($rest)*);
    };

    (@item ($($delegate:tt)*) as $type:ident => ($($sig:tt)*) -> $ty:ty; $($rest:tt)*) => {
        delegate_methods!(@emit ($($delegate)*) as $type => $($sig)* -> $ty);
        delegate_methods!(@item ($($delegate)*) as $type => () $($rest)*);
    };

    (@item ($($delegate:tt)*) as $type:ident => ($($sig:tt)*) -> $ty:ty where $($rest:tt)*) => {
        delegate_methods!(@item ($($delegate)*) as $type => ($($sig)* -> $ty where) $($rest)*);
    };

    (@item ($($delegate:tt)*) as $type:ident => ($($sig:tt)*) ; $($rest:tt)*) => {
        delegate_methods!(@emit ($($delegate)*) as $type => $($sig)*);
        delegate_methods!(@item ($($delegate)*) as $type => () $($rest)*);
    };

    (@item ($($delegate:tt)*) as $type:ident => ($($sig:tt)*) $tt:tt $($rest:tt)*) => {
        delegate_methods!(@item ($($delegate)*) as $type => ($($sig)* $tt) $($rest)*);
    };

    (@emit ($($delegate:tt)*) as $type:ident =>
        $(#[$meta:meta])*
        pub fn $name:ident $(<$($gen:tt),*>)? (&mut self $(, $arg:ident : $arg_ty:ty)*) $($rest:tt)*
    ) => {
        $(#[$meta])*
        #[doc = ""]
        #[doc = concat!(" See [`", stringify!($type), "::", stringify!($name), "`].")]
        pub fn $name $(<$($gen),*>)? (&mut self $(, $arg: $arg_ty)*) $($rest)* {
            self.$($delegate)*.$name($($arg),*)
        }
    };

    (@emit ($($delegate:tt)*) as $type:ident =>
        $(#[$meta:meta])*
        pub unsafe fn $name:ident $(<$($gen:tt),*>)? (&mut self $(, $arg:ident : $arg_ty:ty)*) $($rest:tt)*
    ) => {
        $(#[$meta])*
        #[doc = ""]
        #[doc = concat!(" See [`", stringify!($type), "::", stringify!($name), "`].")]
        #[doc = ""]
        #[doc = "## Safety"]
        #[doc = ""]
        #[doc = concat!(" See [`", stringify!($type), "::", stringify!($name), "`] for safety requirements.")]
        pub unsafe fn $name $(<$($gen),*>)? (&mut self $(, $arg: $arg_ty)*) $($rest)* {
            unsafe { self.$($delegate)*.$name($($arg),*) }
        }
    };

    (@emit ($($delegate:tt)*) as $type:ident; $($bad:tt)*) => {
        compile_error!("delegate_methods: invalid method declaration");
    };
}

pub(crate) use {
    default_impl_ref_observe, delegate_methods, spec_impl_observe, spec_impl_observe_from_ref, spec_impl_ref_observe,
};
