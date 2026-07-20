#[cfg(loom)]
pub use loom::sync::{AtomicUsize, Ordering};

#[cfg(not(loom))]
pub use std::sync::{AtomicUsize, Ordering};

#[cfg(loom)]
pub use loom::thread;

#[cfg(not(loom))]
pub use std::thread;
