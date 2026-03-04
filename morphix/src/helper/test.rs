macro_rules! __mutation_path {
    (@munch [$($segments:expr),*]) => {
        vec![$($segments),*]
    };
    (@munch [$($segments:expr),*] . $name:ident $($rest:tt)*) => {
        __mutation_path!(@munch [$($segments,)* $crate::PathSegment::from(stringify!($name))] $($rest)*)
    };
    (@munch [$($segments:expr),*] . $n:literal $($rest:tt)*) => {
        __mutation_path!(@munch [$($segments,)* $crate::PathSegment::Positive($n)] $($rest)*)
    };
    (@munch [$($segments:expr),*] [- $n:literal] $($rest:tt)*) => {
        __mutation_path!(@munch [$($segments,)* $crate::PathSegment::Negative($n)] $($rest)*)
    };
    (_) => {
        vec![]
    };
    (_ $($rest:tt)+) => {
        __mutation_path!(@munch [] $($rest)+)
    };
}

macro_rules! replace {
    (@parse [$($path:tt)*], $value:expr) => {
        $crate::Mutation {
            path: __mutation_path!($($path)*).into(),
            kind: $crate::MutationKind::Replace($value),
        }
    };
    (@parse [$($path:tt)*] $next:tt $($rest:tt)*) => {
        replace!(@parse [$($path)* $next] $($rest)*)
    };
    ($($all:tt)*) => { replace!(@parse [] $($all)*) };
}

#[cfg(feature = "append")]
macro_rules! append {
    (@parse [$($path:tt)*], $value:expr) => {
        $crate::Mutation {
            path: __mutation_path!($($path)*).into(),
            kind: $crate::MutationKind::Append($value),
        }
    };
    (@parse [$($path:tt)*] $next:tt $($rest:tt)*) => {
        append!(@parse [$($path)* $next] $($rest)*)
    };
    ($($all:tt)*) => { append!(@parse [] $($all)*) };
}

#[cfg(feature = "truncate")]
macro_rules! truncate {
    (@parse [$($path:tt)*], $value:expr) => {
        $crate::Mutation {
            path: __mutation_path!($($path)*).into(),
            kind: $crate::MutationKind::Truncate($value),
        }
    };
    (@parse [$($path:tt)*] $next:tt $($rest:tt)*) => {
        truncate!(@parse [$($path)* $next] $($rest)*)
    };
    ($($all:tt)*) => { truncate!(@parse [] $($all)*) };
}

#[cfg(feature = "delete")]
macro_rules! delete {
    (@parse [$($path:tt)*]) => {
        $crate::Mutation {
            path: __mutation_path!($($path)*).into(),
            kind: $crate::MutationKind::Delete,
        }
    };
    (@parse [$($path:tt)*] $next:tt $($rest:tt)*) => {
        delete!(@parse [$($path)* $next] $($rest)*)
    };
    ($($all:tt)*) => { delete!(@parse [] $($all)*) };
}

macro_rules! batch {
    (@parse [$($path:tt)*], $($items:expr),* $(,)?) => {
        $crate::Mutation {
            path: __mutation_path!($($path)*).into(),
            kind: $crate::MutationKind::Batch(vec![$($items),*]),
        }
    };
    (@parse [$($path:tt)*] $next:tt $($rest:tt)*) => {
        batch!(@parse [$($path)* $next] $($rest)*)
    };
    ($($all:tt)*) => { batch!(@parse [] $($all)*) };
}

#[cfg(feature = "append")]
pub(crate) use append;
#[cfg(feature = "delete")]
pub(crate) use delete;
#[cfg(feature = "truncate")]
pub(crate) use truncate;
pub(crate) use {__mutation_path, batch, replace};
