use std::ops::{Index, RangeFrom};

use serde::Serialize;

use crate::helper::{AsDeref, Unsigned};
use crate::observe::{DebugHandler, DefaultSpec, GeneralHandler, GeneralObserver, RefObserve, SerializeHandler};
use crate::{Adapter, Mutation, MutationKind};

trait Len {
    fn len(&self) -> usize;
}

impl Len for str {
    fn len(&self) -> usize {
        str::len(self)
    }
}

impl<T> Len for [T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }
}

pub struct UnsizeRefHandler<'a, T: ?Sized> {
    ptr: Option<&'a T>,
}

impl<'a, T: ?Sized> GeneralHandler for UnsizeRefHandler<'a, T> {
    type Target = &'a T;
    type Spec = DefaultSpec;

    #[inline]
    fn uninit() -> Self {
        Self { ptr: None }
    }

    #[inline]
    fn observe(value: &&'a T) -> Self {
        Self { ptr: Some(value) }
    }

    #[inline]
    fn deref_mut(&mut self) {}
}

impl<'a, T> SerializeHandler for UnsizeRefHandler<'a, T>
where
    T: Len + Index<RangeFrom<usize>, Output = T> + Serialize + ?Sized,
{
    unsafe fn flush<A: Adapter>(&mut self, new_value: &&'a T) -> Result<Option<Mutation<A::Value>>, A::Error> {
        let old_value = unsafe { self.ptr.unwrap_unchecked() };
        if !std::ptr::addr_eq(*new_value, old_value) {
            return Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(new_value)?),
            }));
        }
        let old_len = old_value.len();
        let new_len = new_value.len();
        if new_len < old_len {
            #[cfg(feature = "truncate")]
            return Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Truncate(old_value[new_len..].len()),
            }));
            #[cfg(not(feature = "truncate"))]
            return Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(new_value)?),
            }));
        }
        if new_len > old_len {
            #[cfg(feature = "append")]
            return Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Append(A::serialize_value(&new_value[old_len..])?),
            }));
            #[cfg(not(feature = "append"))]
            return Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(new_value)?),
            }));
        }
        Ok(None)
    }
}

impl<'a, T: ?Sized> DebugHandler for UnsizeRefHandler<'a, T> {
    const NAME: &'static str = "UnsizedRefHandler";
}

impl RefObserve for str {
    type Observer<'a, 'ob, S, D>
        = GeneralObserver<'ob, UnsizeRefHandler<'a, str>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = &'a Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

impl<T> RefObserve for [T] {
    type Observer<'a, 'ob, S, D>
        = GeneralObserver<'ob, UnsizeRefHandler<'a, [T]>, S, D>
    where
        Self: 'ob,
        T: 'a,
        D: Unsigned,
        S: AsDeref<D, Target = &'a Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::MutationKind;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn test_str_ref_replace() {
        const A: &str = "hello world 1";
        const B: &str = "hello world 2";
        let mut a = &A[0..12];
        let mut ob = a.__observe();
        **ob = &B[0..12];
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("hello world ")));
    }

    #[test]
    fn test_str_ref_eq() {
        const A: &str = "hello world";
        let mut a = A;
        let mut ob = a.__observe();
        **ob = &A[0..];
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn test_str_ref_append() {
        const A: &str = "hello world";
        let mut a = &A[0..5];
        let mut ob = a.__observe();
        **ob = A;
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.unwrap().kind == MutationKind::Append(json!(" world")));
    }

    #[test]
    fn test_str_ref_truncate() {
        const A: &str = "hello world";
        let mut a = A;
        let mut ob = a.__observe();
        **ob = &A[0..5];
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.unwrap().kind == MutationKind::Truncate(6));
    }
}
