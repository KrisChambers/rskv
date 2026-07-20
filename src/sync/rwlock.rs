use crate::sync::atomic::{
    AtomicBool, AtomicUsize,
    Ordering::{Acquire, Release},
    spin_loop
};
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

// How do we think about Send + Sync traits?
pub struct RwLock<T> {
    // The value
    // We are building a container for doing interior immutability
    // Using an UnsafeCell here lets us opt out of Rust's checks
    // So we have to provide an api to enforce them.
    value: UnsafeCell<T>, // This needs to be able to be passed around as a mutable reference.
    // Is there a writer?
    writing: AtomicBool, // Binary semaphore for deciding when we can write.

    // Count of readers currently active?
    readers: AtomicUsize,
}

// Something can safely be Send unless it shares mutable state with something else WITHOUT enforcing
// exclusive access. - The whole point here is that we are enforcing exclusive access to the writer
// thread.
unsafe impl<T> Send for RwLock<T> where T: Send {}

// Something can safely be Sync if it is enforced that you can't write to the object reference while it could
// be read or written to from another reference.
// This is enforced by the aquisition of the RWLock's read method
unsafe impl<T> Sync for RwLock<T> where T: Sync {}

pub struct ReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

unsafe impl<'a, T> Send for ReadGuard<'a, T> where T: Sync {}
unsafe impl<'a, T> Sync for ReadGuard<'a, T> where T: Sync {}

pub struct WriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

// WriteGuard provides &mut T. So it will require T to be Send + Sync
unsafe impl<'a, T> Sync for WriteGuard<'a, T> where T: Send + Sync {}
unsafe impl<'a, T> Send for WriteGuard<'a, T> where T: Send + Sync {}

impl<T> RwLock<T> {
    /// Create a new read-write lock for a value
    pub fn new(value: T) -> Self {
        let writing = AtomicBool::from(false);
        let readers = AtomicUsize::from(0);

        RwLock {
            value: UnsafeCell::new(value),
            writing,
            readers,
        }
    }

    /// Get a read lock
    pub fn read(&self) -> ReadGuard<'_, T> {
        loop {
            // 1. Wait until we see that there is nothing writing
            while self.writing.load(Acquire) {
                spin_loop();
            }

            // 2. Increment the number of readers to signal to
            // Anything that is trying to read that there are readers
            self.readers.fetch_add(1, Acquire);

            // 3. If no writers got in
            // before we incremented readers then we leave the loop
            if !self.writing.load(Acquire) {
                break;
            }

            // 4. Otherwise backoff and try again.
            self.readers.fetch_sub(1, Release);
        }

        ReadGuard { lock: self }
    }

    /// Get a write lock
    pub fn write(&self) -> WriteGuard<'_, T> {
        // If we don't already have a writer
        loop {
            // 1. Are there any readers?
            while self.readers.load(Acquire) > 0 {
                spin_loop();
            }

            match self
                .writing
                .compare_exchange_weak(false, true, Acquire, Acquire)
            {
                // If there are no writers
                Ok(_) => {
                    // Check no readers picked up a lock
                    if self.readers.load(Acquire) == 0 {
                        break;
                    } else {
                        // A reader grabbed a lock, so we back off and try again.
                        self.writing.store(false, Release);
                        continue;
                    }
                }
                // If there are any writers, then we continue
                Err(_) => continue,
            }
        }

        // If we have no readers
        while self.readers.load(Acquire) > 0 {
            spin_loop();
        }

        WriteGuard { lock: self }
    }

    /// Try to obtain a read lock
    pub fn try_read(&self) -> Option<ReadGuard<'_, T>> {
        todo!();
        // Some(ReadGuard {value: &self.value})
    }

    /// Try to obtain a write lock
    pub fn try_write(&self) -> Option<WriteGuard<'_, T>> {
        todo!();
        // Some(WriteGuard {value: &mut self.value})
    }
}

impl<'a, T> Deref for ReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.lock.value.get().as_ref_unchecked() }
    }
}

impl<'a, T> Deref for WriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.value.get() }
    }
}

impl<'a, T> DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<'a, T> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        // SAFETY: `self` is pinned till after dropped.
        // unsafe { Drop::pin_drop(std::pin::Pin::new_unchecked(self)) }
        self.lock.writing.store(false, Release);
    }
}

impl<'a, T> Drop for ReadGuard<'a, T> {
    fn drop(&mut self) {
        // SAFETY: `self` is pinned till after dropped.
        // unsafe { Drop::pin_drop(std::pin::Pin::new_unchecked(self)) }
        self.lock.readers.fetch_sub(1, Release);
    }
}

#[cfg(all(test, not(feature = "shuttle")))]
mod unit_test {
    use super::*;
    use std::thread;
    use std::sync::Arc;
    use proptest::prelude::*;

    #[test]
    fn read_lock_resturns_value() {
        let lock = RwLock::new(1);
        let value = *lock.read();

        assert_eq!(value, 1);
    }

    #[test]
    fn write_lock_returns_value() {
        let lock = RwLock::new(1);
        let value = *lock.write();

        assert_eq!(value, 1);
    }

    #[test]
    fn read_sees_written_value() {
        let lock = RwLock::new(1);

        {
            let mut g = lock.write();
            (*g) = 10;
        }

        {
            let r = lock.read();
            assert_eq!(*r, 10);
        }
    }

    #[test]
    fn write_lock_waits_for_readers_to_drop() {
        let lock = Arc::new(RwLock::new(0));
        let reader = lock.read();

        let thread_lock = lock.clone();
        let w = thread::spawn(move || {
            let mut writer = thread_lock.write();

            *writer = 100;
        });

        assert_eq!(*reader, 0);
        // NOTE: if the join happens here we are stuck waiting to drop the reader.
        drop(reader);
        w.join().unwrap();

        let reader = lock.read();

        assert_eq!(*reader, 100);
    }

    #[test]
    #[cfg_attr(miri, ignore = "too slow under Miri")]
    fn readers_waiting_for_writer() {
        proptest!(|(readers in 1usize..=4)| {
            let read_values: Arc<RwLock<Vec<usize>>> = Arc::new(RwLock::new(vec![]));
            let lock = Arc::new(RwLock::new(0));
            let mut writer = lock.write();

            let mut handles = vec![];
            for _ in 0..readers {
                let input = lock.clone();
                let out = read_values.clone();

                let h = thread::spawn(move ||{
                    let mut output = out.write();
                    let value = input.read();

                    output.push(*value);
                });

                handles.push(h);
            }

            assert_eq!(*writer, 0);
            *writer = 2;
            drop(writer);
            for h in handles {
                h.join().unwrap();
            }

            let reader = read_values.read();
            for value in reader.iter() {
                assert_eq!(*value, 2);
            }
        });


    }

    #[test]
    fn drop_while_waiting() {
        let lock = Arc::new(RwLock::new(0));
        let reader = lock.read();

        let thread_lock = lock.clone();
        let w = thread::spawn(move || {
            let mut writer = thread_lock.write();

            *writer = 100;
        });

        assert_eq!(*reader, 0);

    }
}

#[cfg(all(feature = "shuttle", test))]
mod shuttle_test {

    use super::*;
    use shuttle::sync::Arc;

    #[test]
    fn loom_catches_writer_and_reader_overlap() {
        shuttle::check_random(||{

            let lock = Arc::new(RwLock::new(0));

            let l1 = lock.clone();
            let t1 = shuttle::thread::spawn(move || {
                let mut guard = l1.write();
                // Writer sees an even value and sets it to *guard + 1,
                // which must always be odd while the write guard is held.
                let value = *guard;
                assert_eq!(value % 2, 0, "writer observed an odd value");
                *guard = value + 1;
                // We are adding twice here so there is an opportunity
                // for the reader to read 1 if the locks are not working.
                *guard += 1;
                assert_eq!(value % 2, 0, "writer observed an odd value");
            });

            let t2 = shuttle::thread::spawn(move || {
                let guard = lock.read();
                // Reader must never see an odd value; only the writer
                // produces odd values, and they must be exclusive.
                let value = *guard;
                assert_eq!(value % 2, 0, "reader observed an odd value");
            });

            t1.join().unwrap();
            t2.join().unwrap();
        }, 1000);
    }
}
