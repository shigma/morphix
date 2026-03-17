//! Observer implementations for collection types in [`std::ffi`] or [`std::path`].

mod c_str;
mod c_string;
mod os_str;
mod os_string;
mod path;
mod path_buf;

pub use c_str::CStrObserver;
pub use os_str::OsStrObserver;
pub use path::PathObserver;
