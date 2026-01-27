use std::mem::MaybeUninit;
use std::sync::atomic::Ordering;

use crate::builtin::{DebugHandler, GeneralHandler, GeneralObserver, ReplaceHandler};
use crate::helper::{AsDeref, AsDerefMut, Unsigned};
use crate::observe::{DefaultSpec, Observe, RefObserve};

pub trait Atomic {
    type Value: Copy + PartialEq;

    fn load(&self, ordering: Ordering) -> Self::Value;
}

pub struct AtomicHandler<T>
where
    T: Atomic + ?Sized,
{
    snapshot: MaybeUninit<T::Value>,
}

impl<T> GeneralHandler for AtomicHandler<T>
where
    T: Atomic + ?Sized,
{
    type Target = T;
    type Spec = DefaultSpec;

    #[inline]
    fn uninit() -> Self {
        Self {
            snapshot: MaybeUninit::uninit(),
        }
    }

    #[inline]
    fn observe(value: &T) -> Self {
        Self {
            snapshot: MaybeUninit::new(value.load(Ordering::Relaxed)),
        }
    }

    #[inline]
    fn deref_mut(&mut self) {}
}

impl<T> ReplaceHandler for AtomicHandler<T>
where
    T: Atomic + ?Sized,
{
    #[inline]
    fn flush_replace(&mut self, value: &T) -> bool {
        // SAFETY: `ReplaceHandler::flush_replace` is only called in `Observer::flush_unchecked`, where the
        // observer is assumed to contain a valid pointer
        value.load(Ordering::Relaxed) != unsafe { self.snapshot.assume_init() }
    }
}

impl<T> DebugHandler for AtomicHandler<T>
where
    T: Atomic + ?Sized,
{
    const NAME: &'static str = "AtomicObserver";
}

macro_rules! impl_atomic {
    ($($ident:ident => $output:ty),* $(,)?) => {
        $(
            impl Atomic for std::sync::atomic::$ident {
                type Value = $output;

                #[inline]
                fn load(&self, ordering: Ordering) -> $output {
                    self.load(ordering)
                }
            }

            impl Observe for std::sync::atomic::$ident {
                type Observer<'ob, S, D>
                    = GeneralObserver<'ob, AtomicHandler<S::Target>, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

                type Spec = DefaultSpec;
            }

            impl RefObserve for std::sync::atomic::$ident {
                type Observer<'ob, S, D>
                    = GeneralObserver<'ob, AtomicHandler<S::Target>, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDeref<D, Target = Self> + ?Sized + 'ob;

                type Spec = DefaultSpec;
            }
        )*
    };
}

impl_atomic! {
    AtomicBool => bool,
    AtomicU8 => u8,
    AtomicU16 => u16,
    AtomicU32 => u32,
    AtomicU64 => u64,
    AtomicUsize => usize,
    AtomicI8 => i8,
    AtomicI16 => i16,
    AtomicI32 => i32,
    AtomicI64 => i64,
    AtomicIsize => isize,
}
