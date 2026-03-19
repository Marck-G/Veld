//! Veld Resolver: Dependency resolution and source cache management.
//!
//! This crate provides:
//! - Filesystem utilities for the local source cache (`filesystem`)
//! - Cache manager for dependency locations and search (`cache`)
//! - Dependency resolution via PubGrub (`provider`) — future
//! - DAG construction and topological sort (`graph`) — future

pub mod cache;
pub mod filesystem;

// Future modules (to be implemented):
// pub mod provider;
// pub mod graph;
