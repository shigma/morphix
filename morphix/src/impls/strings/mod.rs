//! Observer implementations for string types.

mod c_str;
mod c_string;
#[cfg(any(unix, windows))]
mod os_str;
#[cfg(any(unix, windows))]
mod os_string;
mod path;
mod path_buf;
mod str;
mod string;

#[cfg(any(unix, windows))]
pub use os_str::OsStrObserver;
#[cfg(any(unix, windows))]
pub use os_string::OsStringObserver;
pub use path::PathObserver;
pub use path_buf::PathBufObserver;
pub use str::StrObserver;
pub use string::StringObserver;
