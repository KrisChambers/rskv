
#[cfg(all(feature = "shuttle", test))]
pub (crate) use shuttle::sync::atomic::{AtomicUsize, AtomicBool, Ordering};


#[cfg(all(feature = "shuttle", test))]
pub (crate) use shuttle::hint::spin_loop;

#[cfg(not(all(feature = "shuttle", test)))]
pub (crate) use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

#[cfg(not(all(feature = "shuttle", test)))]
pub (crate) use std::hint::spin_loop;

