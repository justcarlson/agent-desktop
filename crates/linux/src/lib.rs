#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod actions;
mod adapter;
mod input;
mod system;
#[allow(dead_code, unused_imports)]
mod tree;

pub use adapter::LinuxAdapter;
