//! Observer implementation for [`str`].

use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::slice::SliceIndex;

use crate::general::{Unsize, UnsizeObserver};
use crate::helper::macros::delegate_methods;
use crate::helper::shallow::{ShallowDelegate, ShallowMut};
use crate::helper::{AsDeref, AsDerefMut, Invalidate, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::impls::strings::string::StringObserverState;
use crate::mutation::Mutations;
use crate::observe::{DefaultSpec, Observe, Observer, RefObserve, SerializeObserver};

/// Trait for managing the internal state of a [`StrObserver`].
pub trait StrObserverState: Invalidate<str> + Sized {
    /// Creates state observing the given `str`.
    fn observe(value: &str) -> Self;
}

/// Flush logic for str-backed observer state, parameterized by `S` and `D`.
pub trait StrSerializeObserverState<S: ?Sized, D>: Invalidate<str> {
    /// Consumes the accumulated mutation state and returns the collected [`Mutations`].
    ///
    /// Must fully reset internal state so an immediately subsequent call returns empty.
    fn flush(&mut self, ptr: &mut Pointer<S>) -> Mutations;
}

/// Observer implementation for [`str`].
pub struct StrObserver<'ob, V, S: ?Sized, D = Zero> {
    pub(super) ptr: Pointer<S>,
    pub(super) state: V,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, V, S: ?Sized, D> Deref for StrObserver<'ob, V, S, D> {
    type Target = Pointer<S>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, V, S: ?Sized, D> DerefMut for StrObserver<'ob, V, S, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::ptr::from_mut(self).expose_provenance();
        Pointer::invalidate(&mut self.ptr);
        &mut self.ptr
    }
}

impl<'ob, V, S: ?Sized, D> QuasiObserver for StrObserver<'ob, V, S, D>
where
    V: Invalidate<str>,
    D: Unsigned,
    S: AsDeref<D, Target = str>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    fn invalidate(this: &mut Self) {
        Invalidate::invalidate(&mut this.state, (*this.ptr).as_deref());
    }
}

impl<'ob, V, S: ?Sized, D> Observer for StrObserver<'ob, V, S, D>
where
    V: StrObserverState,
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
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

impl<'ob, V, S: ?Sized, D> SerializeObserver for StrObserver<'ob, V, S, D>
where
    V: StrSerializeObserverState<S, D>,
    D: Unsigned,
    S: AsDeref<D, Target = str>,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        this.state.flush(&mut this.ptr)
    }
}

impl<'ob, V, S: ?Sized, D> StrObserver<'ob, V, S, D>
where
    V: Invalidate<()> + Invalidate<str>,
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
{
    fn nonempty_mut(&mut self) -> &mut str {
        if (*self).untracked_ref().is_empty() {
            self.untracked_mut()
        } else {
            self.tracked_mut()
        }
    }

    delegate_methods! { nonempty_mut() as str =>
        pub fn as_mut_ptr(&mut self) -> *mut u8;
        pub fn make_ascii_uppercase(&mut self);
        pub fn make_ascii_lowercase(&mut self);
    }

    /// See [`str::as_bytes_mut`].
    ///
    /// ## Safety
    ///
    /// See [`str::as_bytes_mut`] for safety requirements.
    pub unsafe fn as_bytes_mut(&mut self) -> ShallowMut<'_, [u8], V> {
        let inner = unsafe { (*self.ptr).as_deref_mut().as_bytes_mut() };
        ShallowMut::new(inner, &raw mut self.state)
    }

    /// See [`str::get_mut`].
    pub fn get_mut<I: SliceIndex<str, Output = str>>(
        &mut self,
        i: I,
    ) -> Option<StrObserver<'_, ShallowDelegate<V>, str>> {
        let output = (*self.ptr).as_deref_mut().get_mut(i)?;
        let state = ShallowDelegate::new(&raw mut self.state);
        let ob = StrObserver {
            ptr: Pointer::new(output),
            state,
            phantom: PhantomData,
        };
        Pointer::register_state::<_, Zero>(&ob.ptr, &ob.state);
        Some(ob)
    }

    /// See [`str::get_unchecked_mut`].
    ///
    /// ## Safety
    ///
    /// See [`str::get_unchecked_mut`] for safety requirements.
    pub unsafe fn get_unchecked_mut<I: SliceIndex<str, Output = str>>(
        &mut self,
        i: I,
    ) -> StrObserver<'_, ShallowDelegate<V>, str> {
        let output = unsafe { (*self.ptr).as_deref_mut().get_unchecked_mut(i) };
        let state = ShallowDelegate::new(&raw mut self.state);
        let ob = StrObserver {
            ptr: Pointer::new(output),
            state,
            phantom: PhantomData,
        };
        Pointer::register_state::<_, Zero>(&ob.ptr, &ob.state);
        ob
    }

    /// See [`str::split_at_mut`].
    pub fn split_at_mut(
        &mut self,
        mid: usize,
    ) -> (
        StrObserver<'_, ShallowDelegate<V>, str>,
        StrObserver<'_, ShallowDelegate<V>, str>,
    ) {
        let state = &raw mut self.state;
        let (left, right) = (*self.ptr).as_deref_mut().split_at_mut(mid);
        let left_ob = StrObserver {
            ptr: Pointer::new(left),
            state: ShallowDelegate::new(state),
            phantom: PhantomData,
        };
        Pointer::register_state::<_, Zero>(&left_ob.ptr, &left_ob.state);
        let right_ob = StrObserver {
            ptr: Pointer::new(right),
            state: ShallowDelegate::new(state),
            phantom: PhantomData,
        };
        Pointer::register_state::<_, Zero>(&right_ob.ptr, &right_ob.state);
        (left_ob, right_ob)
    }

    /// See [`str::split_at_mut_checked`].
    #[allow(clippy::type_complexity)]
    pub fn split_at_mut_checked(
        &mut self,
        mid: usize,
    ) -> Option<(
        StrObserver<'_, ShallowDelegate<V>, str>,
        StrObserver<'_, ShallowDelegate<V>, str>,
    )> {
        let state = &raw mut self.state;
        let (left, right) = (*self.ptr).as_deref_mut().split_at_mut_checked(mid)?;
        let left_ob = StrObserver {
            ptr: Pointer::new(left),
            state: ShallowDelegate::new(state),
            phantom: PhantomData,
        };
        Pointer::register_state::<_, Zero>(&left_ob.ptr, &left_ob.state);
        let right_ob = StrObserver {
            ptr: Pointer::new(right),
            state: ShallowDelegate::new(state),
            phantom: PhantomData,
        };
        Pointer::register_state::<_, Zero>(&right_ob.ptr, &right_ob.state);
        Some((left_ob, right_ob))
    }
}

impl<'ob, V, S: ?Sized, D, I> Index<I> for StrObserver<'ob, V, S, D>
where
    V: Invalidate<str>,
    D: Unsigned,
    S: AsDeref<D, Target = str>,
    I: SliceIndex<str>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.untracked_ref().index(index)
    }
}

impl<'ob, V, S: ?Sized, D, I> IndexMut<I> for StrObserver<'ob, V, S, D>
where
    V: Invalidate<str>,
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
    I: SliceIndex<str>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.tracked_mut().index_mut(index)
    }
}

impl<'ob, V, S: ?Sized, D> Debug for StrObserver<'ob, V, S, D>
where
    V: Invalidate<str>,
    D: Unsigned,
    S: AsDeref<D, Target = str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StrObserver").field(&self.untracked_ref()).finish()
    }
}

impl<'ob, V, S: ?Sized, D> Display for StrObserver<'ob, V, S, D>
where
    V: Invalidate<str>,
    D: Unsigned,
    S: AsDeref<D, Target = str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.untracked_ref(), f)
    }
}

macro_rules! generic_impl_cmp {
    ($(impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? V, S: ?Sized, D> PartialEq<$ty> for StrObserver<'ob, V, S, D>
            where
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: PartialEq<$ty>,
            {
                fn eq(&self, other: &$ty) -> bool {
                    (***self).as_deref().eq(other)
                }
            }

            impl<'ob, $($($gen)*,)? V, S: ?Sized, D> PartialOrd<$ty> for StrObserver<'ob, V, S, D>
            where
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: PartialOrd<$ty>,
            {
                fn partial_cmp(&self, other: &$ty) -> Option<std::cmp::Ordering> {
                    (***self).as_deref().partial_cmp(other)
                }
            }
        )*
    };
}

generic_impl_cmp! {
    impl _ for str;
    impl _ for String;
    impl _ for std::ffi::OsStr;
    impl _ for std::ffi::OsString;
    impl _ for std::path::Path;
    impl _ for std::path::PathBuf;
    impl ['a] _ for std::borrow::Cow<'a, str>;
}

impl Unsize for str {
    type Slice = Self;

    fn len(&self) -> usize {
        self.chars().count()
    }

    fn range_from(&self, from: usize) -> &Self::Slice {
        &self[from..]
    }
}

impl Observe for str {
    type Observer<'ob, S, D>
        = StrObserver<'ob, StringObserverState, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

impl RefObserve for str {
    type Observer<'ob, S, D>
        = UnsizeObserver<'ob, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

#[cfg(test)]
mod tests {
    use morphix_test_utils::*;
    use serde_json::json;

    use super::*;
    use crate::adapter::Json;
    use crate::observe::SerializeObserverExt;

    #[test]
    fn split_at_mut() {
        let mut s = String::from("hello world");
        let mut ob: StrObserver<'_, StringObserverState, str> = Observer::observe(&mut s[..]);
        let (mut left, mut right) = ob.split_at_mut(5);
        // SAFETY: ASCII-for-ASCII replacement preserves utf-8.
        unsafe {
            left.as_bytes_mut()[0] = b'H';
            right.as_bytes_mut()[0] = b'_';
            left.as_bytes_mut()[4] = b'O';
            right.as_bytes_mut()[5] = b'D';
        }
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!("HellO_worlD"))));
    }
}
