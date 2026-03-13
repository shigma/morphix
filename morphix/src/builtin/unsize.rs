use std::marker::PhantomData;
use std::ops::{Index, RangeFrom};

use crate::builtin::{DebugHandler, GeneralHandler, GeneralObserver, SerializeHandler};
use crate::helper::{AsDeref, ObserverState, Zero};
use crate::{MutationKind, Mutations};

pub type UnsizeObserver<'ob, S, D = Zero> = GeneralObserver<'ob, UnsizeHandler<<S as AsDeref<D>>::Target>, S, D>;

trait Len {
    fn len(&self) -> usize;
}

impl Len for str {
    fn len(&self) -> usize {
        self.chars().count()
    }
}

impl<T> Len for [T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }
}

pub struct UnsizeHandler<T: ?Sized> {
    addr: usize,
    old_len: usize,
    phantom: PhantomData<*const T>,
}

impl<T: ?Sized> ObserverState for UnsizeHandler<T> {
    type Target = T;

    fn invalidate(_: &mut Self, _: &T) {}
}

impl<T: ?Sized> GeneralHandler for UnsizeHandler<T>
where
    T: Len,
{
    fn uninit() -> Self {
        Self {
            addr: 0,
            old_len: 0,
            phantom: PhantomData,
        }
    }

    fn observe(value: &T) -> Self {
        Self {
            addr: (value as *const T).cast::<u8>().addr(),
            old_len: value.len(),
            phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> SerializeHandler for UnsizeHandler<T>
where
    T: Len + Index<RangeFrom<usize>, Output = T> + serde::Serialize + 'static,
{
    unsafe fn flush(&mut self, new_value: &T) -> Mutations {
        let old_addr = self.addr;
        let old_len = self.old_len;
        self.addr = (new_value as *const T).cast::<u8>().addr();
        let new_len = new_value.len();
        self.old_len = new_len;
        if (new_value as *const T).cast::<u8>().addr() != old_addr {
            return Mutations::replace(new_value);
        }
        if new_len < old_len {
            #[cfg(feature = "truncate")]
            return MutationKind::Truncate(old_len - new_len).into();
            #[cfg(not(feature = "truncate"))]
            return Mutations::replace(new_value);
        }
        if new_len > old_len {
            #[cfg(feature = "append")]
            return Mutations::append(&new_value[old_len..]);
            #[cfg(not(feature = "append"))]
            return Mutations::replace(new_value);
        }
        Mutations::new()
    }
}

impl<T: Len + ?Sized> DebugHandler for UnsizeHandler<T> {
    const NAME: &'static str = "UnsizeObserver";
}

#[cfg(test)]
mod test {
    use morphix_test_utils::*;
    use serde_json::json;

    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn test_str_ref_replace() {
        const A: &str = "hello world 1";
        const B: &str = "hello world 2";
        let mut a = &A[0..12];
        let mut ob = a.__observe();
        ***ob = &B[0..12];
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!("hello world "))));
    }

    #[test]
    fn test_str_ref_eq() {
        const A: &str = "hello world";
        let mut a = A;
        let mut ob = a.__observe();
        ***ob = &A[0..];
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn test_str_ref_append() {
        const A: &str = "hello world";
        let mut a = &A[0..5];
        let mut ob = a.__observe();
        ***ob = A;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!(" world"))));
    }

    #[test]
    fn test_str_ref_truncate() {
        const A: &str = "hello world";
        let mut a = A;
        let mut ob = a.__observe();
        ***ob = &A[0..5];
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 6)));
    }
}
