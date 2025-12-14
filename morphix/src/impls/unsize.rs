use std::ops::{Index, RangeFrom};
use std::ptr::NonNull;

use serde::Serialize;

use crate::helper::{AsDeref, Unsigned};
use crate::observe::{DebugHandler, DefaultSpec, GeneralHandler, GeneralObserver, RefObserve, SerializeHandler};
use crate::{Adapter, Mutation, MutationKind};

trait Len {
    fn len(&self) -> usize;
}

impl Len for str {
    #[inline]
    fn len(&self) -> usize {
        self.chars().count()
    }
}

impl<T> Len for [T] {
    #[inline]
    fn len(&self) -> usize {
        <[T]>::len(self)
    }
}

pub struct UnsizeRefHandler<T, E>
where
    T: AsDeref<E> + ?Sized,
    E: Unsigned,
{
    ptr: Option<NonNull<T::Target>>,
}

impl<T, E> GeneralHandler for UnsizeRefHandler<T, E>
where
    T: AsDeref<E> + ?Sized,
    E: Unsigned,
{
    type Target = T;
    type Spec = DefaultSpec;

    #[inline]
    fn uninit() -> Self {
        Self { ptr: None }
    }

    #[inline]
    fn observe(value: &T) -> Self {
        Self {
            ptr: Some(NonNull::from(value.as_deref())),
        }
    }

    #[inline]
    fn deref_mut(&mut self) {}
}

impl<T, E> SerializeHandler for UnsizeRefHandler<T, E>
where
    T: AsDeref<E, Target: Len + Index<RangeFrom<usize>, Output = T::Target> + Serialize> + ?Sized,
    E: Unsigned,
{
    unsafe fn flush<A: Adapter>(&mut self, new_value: &T) -> Result<Option<Mutation<A::Value>>, A::Error> {
        let new_value = new_value.as_deref();
        let old_value = unsafe {
            self.ptr
                .expect("Pointer should not be null in GeneralHandler::flush")
                .as_ref()
        };
        if !std::ptr::addr_eq(new_value, old_value) {
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

impl<T, E> DebugHandler for UnsizeRefHandler<T, E>
where
    T: AsDeref<E> + ?Sized,
    E: Unsigned,
{
    const NAME: &'static str = "UnsizedRefObserver";
}

impl RefObserve for str {
    type Observer<'ob, S, D, E>
        = GeneralObserver<'ob, UnsizeRefHandler<S::Target, E>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        E: Unsigned,
        S: AsDeref<D> + ?Sized + 'ob, S::Target: AsDeref<E, Target = Self>;

    type Spec = DefaultSpec;
}

impl<T> RefObserve for [T] {
    type Observer<'ob, S, D, E>
        = GeneralObserver<'ob, UnsizeRefHandler<S::Target, E>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        E: Unsigned,
        S: AsDeref<D> + ?Sized + 'ob, S::Target: AsDeref<E, Target = Self>;

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
