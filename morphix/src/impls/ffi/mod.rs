//! Observer implementations for collection types in [`std::ffi`] or [`std::path`].

mod c_str;
mod c_string;
#[cfg(any(unix, windows))]
mod os_str;
#[cfg(any(unix, windows))]
mod os_string;
mod path;
mod path_buf;
pub(crate) mod shallow;
mod str;
mod string;

pub use c_str::CStrObserver;
#[cfg(any(unix, windows))]
pub use os_str::OsStrObserver;
pub use path::PathObserver;
pub use str::StrObserver;
pub use string::StringObserver;
