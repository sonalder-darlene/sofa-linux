//! The core of Sofa for Linux: everything that reads untrusted input (what the user types, the addresses pages
//! ask for) lives here, in plain Rust with no Qt, so it can be tested on its own.
//!
//! The Qt side (`sofa-tv-browser`) only forwards calls to this crate and shows the results.

#![warn(missing_docs)]

pub mod ad_allowlist;
pub mod address;
pub mod ads;
pub mod bookmark_file;
pub mod engine_age;
pub mod folders;
pub mod history_days;
pub mod library;
pub mod popups;
pub mod requests;
pub mod settings;
pub mod sites;
pub mod sqlite;
pub mod start;
pub mod tabs;
