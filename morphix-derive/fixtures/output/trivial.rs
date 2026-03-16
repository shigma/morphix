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
    type Observer<'ob, S, N> = ::morphix::general::ShallowObserver<'ob, S, N>
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
        Vec<T>: ::morphix::general::Snapshot,
    {
        a: <Vec<T> as ::morphix::general::Snapshot>::Snapshot,
    }
    #[automatically_derived]
    impl<T> ::morphix::general::Snapshot for Bar<T>
    where
        Vec<T>: ::morphix::general::Snapshot,
    {
        type Snapshot = BarSnapshot<T>;
        fn to_snapshot(&self) -> Self::Snapshot {
            BarSnapshot {
                a: ::morphix::general::Snapshot::to_snapshot(&self.a),
            }
        }
        #[allow(clippy::match_like_matches_macro)]
        fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
            ::morphix::general::Snapshot::eq_snapshot(&self.a, &snapshot.a)
        }
    }
};
#[rustfmt::skip]
#[automatically_derived]
impl<T> ::morphix::Observe for Bar<T>
where
    Self: ::morphix::general::Snapshot,
{
    type Observer<'ob, S, N> = ::morphix::general::SnapshotObserver<'ob, S, N>
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
impl ::morphix::general::Snapshot for NoopStruct {
    type Snapshot = ();
    fn to_snapshot(&self) {}
    fn eq_snapshot(&self, snapshot: &()) -> bool {
        true
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::Observe for NoopStruct {
    type Observer<'ob, S, N> = ::morphix::general::NoopObserver<'ob, S, N>
    where
        Self: 'ob,
        N: ::morphix::helper::Unsigned,
        S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
    type Spec = ::morphix::observe::SnapshotSpec;
}
