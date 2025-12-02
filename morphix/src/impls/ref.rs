use crate::Observe;
use crate::helper::{AsDerefMut, Unsigned};
use crate::observe::{DefaultSpec, GeneralHandler, GeneralObserver, SnapshotSpec};

pub struct RefHandler<'a, T: ?Sized> {
    ptr: Option<&'a T>,
}

impl<'a, T: ?Sized> Default for RefHandler<'a, T> {
    #[inline]
    fn default() -> Self {
        Self { ptr: None }
    }
}

impl<'a, T: ?Sized> GeneralHandler<&'a T> for RefHandler<'a, T> {
    type Spec = SnapshotSpec;

    #[inline]
    fn on_observe(value: &mut &'a T) -> Self {
        Self { ptr: Some(value) }
    }

    #[inline]
    fn on_deref_mut(&mut self) {}

    #[inline]
    fn on_collect(&self, value: &&'a T) -> bool {
        !std::ptr::eq(*value, unsafe { self.ptr.unwrap_unchecked() })
    }
}

pub struct PartialEqHandler<'a, T: ?Sized> {
    ptr: Option<&'a T>,
}

impl<'a, T: ?Sized> Default for PartialEqHandler<'a, T> {
    #[inline]
    fn default() -> Self {
        Self { ptr: None }
    }
}

impl<'a, T: PartialEq + ?Sized> GeneralHandler<&'a T> for PartialEqHandler<'a, T> {
    type Spec = SnapshotSpec;

    #[inline]
    fn on_observe(value: &mut &'a T) -> Self {
        Self { ptr: Some(value) }
    }

    #[inline]
    fn on_deref_mut(&mut self) {}

    #[inline]
    fn on_collect(&self, value: &&'a T) -> bool {
        *value != unsafe { self.ptr.unwrap_unchecked() }
    }
}

macro_rules! impl_ref_observe {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> Observe for &'a $ty {
                type Observer<'ob, S, D>
                    = GeneralObserver<'ob, RefHandler<'a, $ty>, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

                type Spec = DefaultSpec;
            }
        )*
    };
}

impl_ref_observe! {
    str,
}

macro_rules! impl_partial_eq_observe {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> Observe for &'a $ty {
                type Observer<'ob, S, D>
                    = GeneralObserver<'ob, PartialEqHandler<'a, $ty>, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

                type Spec = DefaultSpec;
            }
        )*
    };
}

impl_partial_eq_observe! {
    usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool, char,
    ::core::net::IpAddr, ::core::net::Ipv4Addr, ::core::net::Ipv6Addr,
    ::core::net::SocketAddr, ::core::net::SocketAddrV4, ::core::net::SocketAddrV6,
    ::core::time::Duration, ::std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    // TODO: enable tests after further implementation
    // use crate::adapter::Json;
    // use crate::observe::{ObserveExt, SerializeObserverExt};

    // #[test]
    // fn test_ptr_eq() {
    //     let a = 42u8;
    //     let b = 42u8;
    //     let mut ptr = &a;
    //     let mut ob = ptr.__observe();
    //     **ob = &a;
    //     let Json(mutation) = ob.collect().unwrap();
    //     assert!(mutation.is_none());
    //     **ob = &b;
    //     let Json(mutation) = ob.collect().unwrap();
    //     assert!(mutation.is_some());
    // }
}
