//! kmatrix-core — Model layer for the K-Matrix Toolkit.
//!
//! Contains domain types, file parsers (XLSX, DBC), search engine,
//! ECU routing resolution, and file-level caching.
//! No HTTP dependencies — this is a pure library.

pub mod cache;
pub mod model;
pub mod parser;
pub mod routing;
pub mod search;

pub use model::*;
pub use parser::ParserRegistry;
pub use search::{build_index, search, SearchFilter, SearchHit};
