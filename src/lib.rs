extern crate bufstream;
extern crate sodiumoxide;

mod packet;
mod stream;
pub mod connection;
pub mod channel;
pub mod transport;

#[cfg(feature = "ffi")]
pub mod api_ffi;

#[macro_export]
macro_rules! error_str {
    ($fmt: expr, $($name: ident), *) => { error_str!(format!($fmt, $($name = $name),*)) };
    ($fmt: expr) => { Error::new(ErrorKind::Other, $fmt) }
}
