use crate::Observe;
use crate::helper::{AsDeref, AsDerefMut, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, RefObserve};

pub mod array;
pub mod deref;
pub mod option;
pub mod slice;
pub mod string;
pub mod tuple;
pub mod unsize;
pub mod vec;

macro_rules! impl_succ_observe {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<T> Observe for $ty
            where
                T: RefObserve + ?Sized,
            {
                type Observer<'ob, S, D>
                    = T::Observer<'ob, S, D, Succ<Zero>>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

                type Spec = DefaultSpec;
            }
        )*
    };
}

macro_rules! impl_succ_ref_observe {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<T> RefObserve for $ty
            where
                T: RefObserve + ?Sized,
            {
                type Observer<'ob, S, D, E>
                    = T::Observer<'ob, S, D, Succ<E>>
                where
                    Self: 'ob,
                    D: Unsigned,
                    E: Unsigned,
                    S: AsDeref<D> + ?Sized + 'ob,
                    S::Target: AsDeref<E, Target = Self>;

                type Spec = DefaultSpec;
            }
        )*
    };
}

impl_succ_observe! {
    &T, std::rc::Rc<T>, std::sync::Arc<T>,
}

impl_succ_ref_observe! {
    Box<T>,
    &T, std::rc::Rc<T>, std::sync::Arc<T>,
}
