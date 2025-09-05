// src/lib.rs
pub mod protogen;
pub mod services;
pub mod storage;
pub mod types;
// Re-export commonly used types for convenience
pub use types::SerdeTimestamp;