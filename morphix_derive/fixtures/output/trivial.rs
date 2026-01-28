#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Foo<T> {
    a: T,
}
#[rustfmt::skip]
#[automatically_derived]
impl<T> ::morphix::Observe for Foo<T> {
    type Observer<'ob, S, N> = ::morphix::builtin::ShallowObserver<'ob, S, N>
    where
        Self: 'ob,
        N: ::morphix::helper::Unsigned,
        S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
    type Spec = ::morphix::observe::DefaultSpec;
}
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Bar<T> {
    a: Vec<T>,
}
#[rustfmt::skip]
const _: () = {
    pub struct BarSnapshot<T>
    where
        Vec<T>: ::morphix::builtin::Snapshot,
    {
        a: <Vec<T> as ::morphix::builtin::Snapshot>::Value,
    }
    #[automatically_derived]
    impl<T> ::morphix::builtin::Snapshot for Bar<T>
    where
        Vec<T>: ::morphix::builtin::Snapshot,
    {
        type Value = BarSnapshot<T>;
        #[inline]
        fn to_snapshot(&self) -> Self::Value {
            BarSnapshot {
                a: ::morphix::builtin::Snapshot::to_snapshot(&self.a),
            }
        }
        #[inline]
        fn eq_snapshot(&self, snapshot: &Self::Value) -> bool {
            ::morphix::builtin::Snapshot::eq_snapshot(&self.a, &snapshot.a)
        }
    }
};
#[rustfmt::skip]
#[automatically_derived]
impl<T> ::morphix::Observe for Bar<T>
where
    Self: ::morphix::builtin::Snapshot,
{
    type Observer<'ob, S, N> = ::morphix::builtin::SnapshotObserver<'ob, S, N>
    where
        Self: 'ob,
        N: ::morphix::helper::Unsigned,
        S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
    type Spec = ::morphix::observe::SnapshotSpec;
}
#[rustfmt::skip]
#[derive(Serialize)]
pub struct NoopStruct {}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::builtin::Snapshot for NoopStruct {
    type Value = ();
    #[inline]
    fn to_snapshot(&self) {}
    #[inline]
    fn eq_snapshot(&self, snapshot: &()) -> bool {
        true
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::Observe for NoopStruct {
    type Observer<'ob, S, N> = ::morphix::builtin::NoopObserver<'ob, S, N>
    where
        Self: 'ob,
        N: ::morphix::helper::Unsigned,
        S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
    type Spec = ::morphix::observe::SnapshotSpec;
}
