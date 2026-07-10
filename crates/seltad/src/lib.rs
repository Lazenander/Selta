//! seltad as a library: everything the binary wires together, exposed so
//! integration tests can run the real router in-process.

pub mod api;
pub mod config;
pub mod settings;
pub mod state;
pub mod stats;
pub mod storage;
pub mod ws;
