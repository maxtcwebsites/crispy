//! Crispy core: a peer-to-peer engine that shares one keyboard, mouse, clipboard and files
//! between computers. Every computer runs the same engine — there is no server and no client.
//! Whichever computer's physical keyboard and mouse are in use drives the others.

pub mod capture;
pub mod clipboard;
pub mod config;
pub mod engine;
pub mod files;
pub mod identity;
pub mod keymap;
pub mod layout;
pub mod net;
pub mod platform;
pub mod protocol;
pub mod state;
pub mod types;
