//! Observer implementation for [`OsStr`].

use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::marker::PhantomData;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

use crate::Mutations;
use crate::general::{DebugHandler, GeneralHandler, GeneralObserver, SerializeHandler};
use crate::helper::macros::delegate_methods;
use crate::helper::shallow::{ShallowInvalidate, ShallowMut};
use crate::helper::{AsDeref, AsDerefMut, Invalidate, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::impls::strings::os_string::OsStringObserverState;
use crate::observe::{DefaultSpec, Observe, Observer, RefObserve, SerializeObserver};

/// Trait for managing the internal state of an [`OsStrObserver`].
pub trait OsStrObserverState: Invalidate<Target = OsStr> + Sized {
    /// Creates state observing the given `OsStr`.
    fn observe(value: &OsStr) -> Self;
}

/// Flush logic for OsStr-backed observer state, parameterized by `S` and `D`.
pub trait OsStrSerializeObserverState<S: ?Sized, D>: Invalidate {
    fn flush(&mut self, ptr: &mut Pointer<S>) -> Mutations;
}

#[cfg(unix)]
pub(super) fn os_str_len(value: &OsStr) -> usize {
    value.as_bytes().len()
}

#[cfg(windows)]
pub(super) fn os_str_len(value: &OsStr) -> usize {
    value.encode_wide().count()
}

/// Observer implementation for [`OsStr`].
pub struct OsStrObserver<'ob, V, S: ?Sized, D = Zero> {
    pub(super) ptr: Pointer<S>,
    pub(super) state: V,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, V, S: ?Sized, D> Deref for OsStrObserver<'ob, V, S, D> {
    type Target = Pointer<S>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, V, S: ?Sized, D> DerefMut for OsStrObserver<'ob, V, S, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::ptr::from_mut(self).expose_provenance();
        Pointer::invalidate(&mut self.ptr);
        &mut self.ptr
    }
}

impl<'ob, V, S: ?Sized, D> QuasiObserver for OsStrObserver<'ob, V, S, D>
where
    V: Invalidate<Target = OsStr>,
    D: Unsigned,
    S: AsDeref<D, Target = OsStr>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    fn invalidate(this: &mut Self) {
        Invalidate::invalidate(&mut this.state, (*this.ptr).as_deref());
    }
}

impl<'ob, V, S: ?Sized, D> Observer for OsStrObserver<'ob, V, S, D>
where
    V: OsStrObserverState,
    D: Unsigned,
    S: AsDerefMut<D, Target = OsStr>,
{
    fn observe(head: &mut Self::Head) -> Self {
        let this = Self {
            state: V::observe(head.as_deref_mut()),
            ptr: Pointer::new(head),
            phantom: PhantomData,
        };
        Pointer::register_state::<_, D>(&this.ptr, &this.state);
        this
    }

    unsafe fn relocate(this: &mut Self, head: &mut Self::Head) {
        Pointer::set(this, head);
    }
}

impl<'ob, V, S: ?Sized, D> SerializeObserver for OsStrObserver<'ob, V, S, D>
where
    V: OsStrSerializeObserverState<S, D, Target = OsStr>,
    D: Unsigned,
    S: AsDeref<D, Target = OsStr>,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        this.state.flush(&mut this.ptr)
    }
}

impl<'ob, V, S: ?Sized, D> OsStrObserver<'ob, V, S, D>
where
    V: ShallowInvalidate + Invalidate<Target = OsStr>,
    D: Unsigned,
    S: AsDerefMut<D, Target = OsStr>,
{
    fn nonempty_mut(&mut self) -> &mut OsStr {
        if (*self).untracked_ref().is_empty() {
            self.untracked_mut()
        } else {
            self.tracked_mut()
        }
    }

    delegate_methods! { nonempty_mut() as OsStr =>
        pub fn make_ascii_uppercase(&mut self);
        pub fn make_ascii_lowercase(&mut self);
    }
}

impl<V: ShallowInvalidate + ?Sized> ShallowMut<'_, OsStr, V> {
    fn nonempty_mut(&mut self) -> &mut OsStr {
        if (*self).untracked_ref().is_empty() {
            self.untracked_mut()
        } else {
            self.tracked_mut()
        }
    }

    delegate_methods! { nonempty_mut() as OsStr =>
        pub fn make_ascii_uppercase(&mut self);
        pub fn make_ascii_lowercase(&mut self);
    }
}

impl<'ob, V, S: ?Sized, D> Debug for OsStrObserver<'ob, V, S, D>
where
    V: Invalidate<Target = OsStr>,
    D: Unsigned,
    S: AsDeref<D, Target = OsStr>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OsStrObserver").field(&self.untracked_ref()).finish()
    }
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? V, S: ?Sized, D> PartialEq<$ty> for OsStrObserver<'ob, V, S, D>
            where
                V: Invalidate<Target = OsStr>,
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: PartialEq<$ty>,
            {
                fn eq(&self, other: &$ty) -> bool {
                    (***self).as_deref().eq(other)
                }
            }
        )*
    };
}

generic_impl_partial_eq! {
    impl _ for str;
    impl _ for String;
    impl _ for OsStr;
    impl _ for OsString;
    impl _ for Path;
    impl _ for PathBuf;
    impl ['a, T] _ for &'a T;
    impl ['a, T: ToOwned] _ for std::borrow::Cow<'a, T>;
}

pub struct OsStrHandler {
    raw_parts: Option<(NonNull<()>, usize)>,
}

impl Invalidate for OsStrHandler {
    type Target = OsStr;

    fn invalidate(&mut self, value: &OsStr) {
        self.raw_parts
            .get_or_insert_with(|| (NonNull::from(value).cast::<()>(), os_str_len(value)));
    }
}

impl GeneralHandler for OsStrHandler {
    fn observe(_: &OsStr) -> Self {
        Self { raw_parts: None }
    }
}

impl SerializeHandler for OsStrHandler {
    unsafe fn flush(&mut self, value: &OsStr) -> Mutations {
        let Some((old_addr, old_len)) = self.raw_parts.take() else {
            return Mutations::new();
        };
        let new_addr = NonNull::from(value).cast::<()>();
        let new_len = os_str_len(value);
        if new_addr != old_addr {
            return Mutations::replace(value);
        }
        if new_len < old_len {
            #[cfg(not(feature = "truncate"))]
            return Mutations::replace(value);
            #[cfg(feature = "truncate")]
            {
                #[cfg(unix)]
                return Mutations::truncate(old_len - new_len).with_prefix("Unix");
                #[cfg(windows)]
                return Mutations::truncate(old_len - new_len).with_prefix("Windows");
            }
        }
        if new_len > old_len {
            #[cfg(not(feature = "append"))]
            return Mutations::replace(value);
            #[cfg(feature = "append")]
            {
                #[cfg(unix)]
                return Mutations::append(&value.as_bytes()[old_len..]).with_prefix("Unix");
                #[cfg(windows)]
                return Mutations::append_owned(value.encode_wide().skip(old_len).collect::<Vec<_>>())
                    .with_prefix("Windows");
            }
        }
        Mutations::new()
    }
}

impl DebugHandler for OsStrHandler {
    const NAME: &'static str = "OsStrHandler";
}

impl Observe for OsStr {
    type Observer<'ob, S, D>
        = OsStrObserver<'ob, OsStringObserverState, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

impl RefObserve for OsStr {
    type Observer<'ob, S, D>
        = GeneralObserver<'ob, OsStrHandler, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}
