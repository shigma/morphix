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
    (@item $delegate:ident $type:ident
        $(#[doc = $doc:expr])*
        pub fn $name:ident $(<$($gen:tt),*>)? (&mut self $(, $arg:ident: $arg_ty:ty)*) $(-> $ret:ty)? $(where { $($where:tt)+ })?;
    ) => {
        $(#[doc = $doc])*
        #[doc = ""]
        #[doc = concat!(" See [`", stringify!($type), "::", stringify!($name), "`].")]
        #[inline]
        pub fn $name $(<$($gen),*>)? (&mut self $(, $arg: $arg_ty)*) $(-> $ret)? $(where $($where)+)? {
            self.$delegate().$name($($arg),*)
        }
    };
    (@item $delegate:ident $type:ident
        $(#[doc = $doc:expr])*
        pub unsafe fn $name:ident $(<$($gen:tt),*>)? (&mut self $(, $arg:ident: $arg_ty:ty)*) $(-> $ret:ty)? $(where { $($where:tt)+ })?;
    ) => {
        $(#[doc = $doc])*
        #[doc = ""]
        #[doc = concat!(" See [`", stringify!($type), "::", stringify!($name), "`].")]
        ///
        /// # Safety
        ///
        #[doc = concat!(" See [`", stringify!($type), "::", stringify!($name), "`] for safety requirements.")]
        #[inline]
        pub unsafe fn $name $(<$($gen),*>)? (&mut self $(, $arg: $arg_ty)*) $(-> $ret)? $(where $($where)+)? {
            unsafe { self.$delegate().$name($($arg),*) }
        }
    };
    ($delegate:ident as $type:ident => $($item:tt)*) => {
        delegate_methods!(@parse $delegate $type [] $($item)*);
    };
    (@parse $delegate:ident $type:ident [$($acc:tt)*]) => {
        $($acc)*
    };
    (@parse $delegate:ident $type:ident [$($acc:tt)*]
        $(#[doc = $doc:expr])*
        pub unsafe fn $name:ident $(<$($gen:tt),*>)? (&mut self $(, $arg:ident: $arg_ty:ty)*) $(-> $ret:ty)? $(where { $($where:tt)+ })?;
        $($rest:tt)*
    ) => {
        delegate_methods!(@parse $delegate $type [
            $($acc)*
            delegate_methods!(@item $delegate $type
                $(#[doc = $doc])*
                pub unsafe fn $name $(<$($gen),*>)? (&mut self $(, $arg: $arg_ty)*) $(-> $ret)? $(where { $($where)+ })?;
            );
        ] $($rest)*);
    };
    (@parse $delegate:ident $type:ident [$($acc:tt)*]
        $(#[doc = $doc:expr])*
        pub fn $name:ident $(<$($gen:tt),*>)? (&mut self $(, $arg:ident: $arg_ty:ty)*) $(-> $ret:ty)? $(where { $($where:tt)+ })?;
        $($rest:tt)*
    ) => {
        delegate_methods!(@parse $delegate $type [
            $($acc)*
            delegate_methods!(@item $delegate $type
                $(#[doc = $doc])*
                pub fn $name $(<$($gen),*>)? (&mut self $(, $arg: $arg_ty)*) $(-> $ret)? $(where { $($where)+ })?;
            );
        ] $($rest)*);
    };
}

pub(crate) use {
    default_impl_ref_observe, delegate_methods, spec_impl_observe, spec_impl_observe_from_ref, spec_impl_ref_observe,
};
