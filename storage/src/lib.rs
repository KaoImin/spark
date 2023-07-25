//! # The Spark Storage Library
//!
//! The Spark Storage Library provides two main components:
//! - The relation database
//! - The sparse merkle tree database

pub mod kvdb;
pub mod relation_db;
pub mod smt;

mod error;

#[cfg(test)]
mod tests;

pub use kvdb::KVDB;
pub use relation_db::RelationDB;
pub use smt::SmtManager;
