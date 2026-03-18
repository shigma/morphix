use std::marker::PhantomData;
use std::ptr::NonNull;

use serde::Serialize;

use crate::general::{DebugHandler, GeneralHandler, GeneralObserver, SerializeHandler};
use crate::helper::{AsDeref, ObserverState, Zero};
use crate::{MutationKind, Mutations};

pub type UnsizeObserver<'ob, S, D = Zero> = GeneralObserver<'ob, UnsizeHandler<<S as AsDeref<D>>::Target>, S, D>;

pub trait Unsize {
    type Slice: ?Sized;

    fn len(&self) -> usize;

    fn range_from(&self, from: usize) -> &Self::Slice;
}

pub struct UnsizeHandler<T: ?Sized> {
    raw_parts: Option<(NonNull<()>, usize)>,
    phantom: PhantomData<*const T>,
}

impl<T: ?Sized> ObserverState for UnsizeHandler<T>
where
    T: Unsize,
{
    type Target = T;

    fn invalidate(this: &mut Self, value: &T) {
        this.raw_parts
            .get_or_insert_with(|| (NonNull::from(value).cast::<()>(), value.len()));
    }
}

impl<T: ?Sized> GeneralHandler for UnsizeHandler<T>
where
    T: Unsize,
{
    fn observe(_: &T) -> Self {
        Self {
            raw_parts: None,
            phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> SerializeHandler for UnsizeHandler<T>
where
    T: Unsize<Slice: Serialize> + Serialize + 'static,
{
    unsafe fn flush(&mut self, value: &T) -> Mutations {
        let Some((old_addr, old_len)) = self.raw_parts.take() else {
            return Mutations::new();
        };
        let new_addr = NonNull::from(value).cast::<()>();
        let new_len = value.len();
        if new_addr != old_addr {
            return Mutations::replace(value);
        }
        if new_len < old_len {
            #[cfg(feature = "truncate")]
            return MutationKind::Truncate(old_len - new_len).into();
            #[cfg(not(feature = "truncate"))]
            return Mutations::replace(value);
        }
        if new_len > old_len {
            #[cfg(feature = "append")]
            return Mutations::append(value.range_from(old_len));
            #[cfg(not(feature = "append"))]
            return Mutations::replace(value);
        }
        Mutations::new()
    }
}

impl<T: Unsize + ?Sized> DebugHandler for UnsizeHandler<T> {
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
