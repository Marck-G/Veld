//! Veld Resolver: Dependency resolution and source cache management.
//!
//! This crate provides:
//! - Filesystem utilities for the local source cache (`filesystem`)
//! - Cache manager for dependency locations and search (`cache`)
//! - Dependency graph with DAG, topological sort, and cycle detection (`graph`)
//! - Dependency resolution via recursive tree walking (`provider`)

pub mod cache;
pub mod filesystem;
pub mod graph;
pub mod provider;

pub use graph::{DependencyGraph, ResolvedPackage};
pub use provider::{DependencyConflict, DependencyResolver, ResolutionResult};
