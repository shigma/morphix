use std::marker::PhantomData;

pub trait Unsigned: 'static {}

pub struct Zero;

impl Unsigned for Zero {}

pub struct Succ<N: Unsigned>(PhantomData<N>);

impl<N: Unsigned> Unsigned for Succ<N> {}
