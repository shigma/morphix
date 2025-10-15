use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PathSegment {
    String(Cow<'static, str>),
    Number(isize),
}

impl From<isize> for PathSegment {
    fn from(n: isize) -> Self {
        Self::Number(n)
    }
}

impl From<&'static str> for PathSegment {
    fn from(s: &'static str) -> Self {
        Self::String(Cow::Borrowed(s))
    }
}

impl From<String> for PathSegment {
    fn from(s: String) -> Self {
        Self::String(Cow::Owned(s))
    }
}

impl From<Cow<'static, str>> for PathSegment {
    fn from(s: Cow<'static, str>) -> Self {
        Self::String(s)
    }
}

impl Display for PathSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathSegment::String(s) => write!(f, ".{s}"),
            PathSegment::Number(n) => write!(f, "[{n}]"),
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Path<const REV: bool>(Vec<PathSegment>);

impl<const REV: bool> Path<REV> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<const REV: bool> From<Vec<PathSegment>> for Path<REV> {
    fn from(mut segments: Vec<PathSegment>) -> Self {
        if REV {
            segments.reverse();
        }
        Self(segments)
    }
}

impl<const REV: bool> Display for Path<REV> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if REV {
            for segment in self.0.iter().rev() {
                write!(f, "{segment}")?;
            }
        } else {
            for segment in self.0.iter() {
                write!(f, "{segment}")?;
            }
        };
        Ok(())
    }
}

impl<const REV: bool> Debug for Path<REV> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Path").field(&self.to_string()).finish()
    }
}

impl<const REV: bool> Deref for Path<REV> {
    type Target = Vec<PathSegment>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const REV: bool> DerefMut for Path<REV> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
