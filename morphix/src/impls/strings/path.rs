//! Observer implementation for [`Path`].

use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

use crate::Mutations;
use crate::general::{DebugHandler, GeneralHandler, GeneralObserver, SerializeHandler};
use crate::helper::shallow::{ShallowDelegate, ShallowInvalidate, ShallowState};
use crate::helper::{AsDeref, AsDerefMut, Invalidate, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::impls::strings::os_str::OsStrObserver;
use crate::observe::{DefaultSpec, Observe, Observer, RefObserve, SerializeObserver};

/// Trait for managing the internal state of a [`PathObserver`].
pub trait PathObserverState: Invalidate<Target = Path> + Sized {
    /// Creates state observing the given `Path`.
    fn observe(value: &Path) -> Self;
}

/// Flush logic for Path-backed observer state, parameterized by `S` and `D`.
pub trait PathSerializeObserverState<S: ?Sized, D>: Invalidate {
    fn flush(&mut self, ptr: &mut Pointer<S>) -> Mutations;
}

/// Observer implementation for [`Path`].
pub struct PathObserver<'ob, V, S: ?Sized, D = Zero> {
    pub(super) ptr: Pointer<S>,
    pub(super) state: V,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, V, S: ?Sized, D> Deref for PathObserver<'ob, V, S, D> {
    type Target = Pointer<S>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, V, S: ?Sized, D> DerefMut for PathObserver<'ob, V, S, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::ptr::from_mut(self).expose_provenance();
        Pointer::invalidate(&mut self.ptr);
        &mut self.ptr
    }
}

impl<'ob, V, S: ?Sized, D> QuasiObserver for PathObserver<'ob, V, S, D>
where
    V: Invalidate<Target = Path>,
    D: Unsigned,
    S: AsDeref<D, Target = Path>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    fn invalidate(this: &mut Self) {
        Invalidate::invalidate(&mut this.state, (*this.ptr).as_deref());
    }
}

impl<'ob, V, S: ?Sized, D> Observer for PathObserver<'ob, V, S, D>
where
    V: PathObserverState,
    D: Unsigned,
    S: AsDerefMut<D, Target = Path>,
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

impl<'ob, V, S: ?Sized, D> SerializeObserver for PathObserver<'ob, V, S, D>
where
    V: PathSerializeObserverState<S, D, Target = Path>,
    D: Unsigned,
    S: AsDeref<D, Target = Path>,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        this.state.flush(&mut this.ptr)
    }
}

impl<'ob, V, S: ?Sized, D> PathObserver<'ob, V, S, D>
where
    V: ShallowInvalidate + Invalidate<Target = Path>,
    D: Unsigned,
    S: AsDerefMut<D, Target = Path>,
{
    /// See [`Path::as_mut_os_str`].
    pub fn as_mut_os_str(&mut self) -> OsStrObserver<'_, ShallowDelegate<OsStr, V>, OsStr> {
        let state = ShallowDelegate::new(&raw mut self.state);
        let os_str = (*self.ptr).as_deref_mut().as_mut_os_str();
        let ob = OsStrObserver {
            state,
            ptr: Pointer::new(os_str),
            phantom: PhantomData,
        };
        Pointer::register_state::<_, Zero>(&ob.ptr, &ob.state);
        ob
    }
}

impl<'ob, V, S: ?Sized, D> Debug for PathObserver<'ob, V, S, D>
where
    V: Invalidate<Target = Path>,
    D: Unsigned,
    S: AsDeref<D, Target = Path>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PathObserver").field(&self.untracked_ref()).finish()
    }
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? V, S: ?Sized, D> PartialEq<$ty> for PathObserver<'ob, V, S, D>
            where
                V: Invalidate<Target = Path>,
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

pub struct PathHandler {
    raw_parts: Option<Option<(NonNull<()>, usize)>>,
}

impl Invalidate for PathHandler {
    type Target = Path;

    fn invalidate(&mut self, value: &Path) {
        self.raw_parts.get_or_insert_with(|| {
            value
                .to_str()
                .map(|str| (NonNull::from(str).cast::<()>(), str.chars().count()))
        });
    }
}

impl GeneralHandler for PathHandler {
    fn observe(_: &Path) -> Self {
        Self { raw_parts: None }
    }
}

impl SerializeHandler for PathHandler {
    unsafe fn flush(&mut self, value: &Path) -> Mutations {
        let (old_addr, old_len) = match self.raw_parts.take() {
            None => return Mutations::new(),
            Some(None) => return Mutations::replace(value),
            Some(Some(parts)) => parts,
        };
        let Some(str) = value.to_str() else {
            return Mutations::replace(value);
        };
        let new_addr = NonNull::from(str).cast::<()>();
        let new_len = str.chars().count();
        if new_addr != old_addr {
            return Mutations::replace(value);
        }
        if new_len < old_len {
            #[cfg(not(feature = "truncate"))]
            return Mutations::replace(value);
            #[cfg(feature = "truncate")]
            return Mutations::truncate(old_len - new_len);
        }
        if new_len > old_len {
            #[cfg(not(feature = "append"))]
            return Mutations::replace(value);
            #[cfg(feature = "append")]
            return Mutations::append(&str[old_len..]);
        }
        Mutations::new()
    }
}

impl DebugHandler for PathHandler {
    const NAME: &'static str = "PathHandler";
}

impl PathObserverState for ShallowState<Path> {
    fn observe(_value: &Path) -> Self {
        ShallowState::new()
    }
}

impl<S: ?Sized, D> PathSerializeObserverState<S, D> for ShallowState<Path>
where
    D: Unsigned,
    S: AsDeref<D, Target = Path>,
{
    fn flush(&mut self, ptr: &mut Pointer<S>) -> Mutations {
        if self.take() {
            Mutations::replace((**ptr).as_deref())
        } else {
            Mutations::new()
        }
    }
}

impl Observe for Path {
    type Observer<'ob, S, D>
        = PathObserver<'ob, ShallowState<Path>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

impl RefObserve for Path {
    type Observer<'ob, S, D>
        = GeneralObserver<'ob, PathHandler, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}
