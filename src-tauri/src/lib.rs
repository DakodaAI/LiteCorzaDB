//! Lite CorzaDB: a dependency-free, embedded database core.
//!
//! The crate deliberately keeps the first storage and SQL surface small. The
//! format is checksummed, writes are atomic, decoding is bounded, and all public
//! operations are safe Rust.

mod codec;
mod database;
mod error;
mod sql;
mod value;

pub use database::{Database, DatabaseStats, QueryResult};
pub use error::{CorzaError, Result};
pub use sql::{parse, Statement};
pub use value::{Column, DataType, Row, Value};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn engine_version() -> &'static str {
    VERSION
}
