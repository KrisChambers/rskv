use std::{hint::{self, spin_loop}, ops::{Deref, DerefMut}, sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed}}, thread, time::Duration};
use std::cell::UnsafeCell;



// How do we think about Send + Sync traits?
pub struct RWLock<T> {
    // The value
    // We are building a container for doing interior immutability
    // Using an UnsafeCell here lets us opt out of Rust's checks
    // So we have to provide an api to enforce them.
    value: UnsafeCell<T>, // This needs to be able to be passed around as a mutable reference.
    // Is there a writer?
    writing: AtomicBool, // Binary semaphore for deciding when we can write.

    // Count of readers currently active?
    readers: AtomicUsize
}

// Something can safely be Send unless it shares mutable state with something else WITHOUT enforcing
// exclusive access. - The whole point here is that we are enforcing exclusive access to the writer
// thread.
unsafe impl<T> Send for RWLock<T> where T: Send {}

// Something can safely be Sync if it is enforced that you can't write to the object reference while it could
// be read or written to from another reference.
// This is enforced by the aquisition of the RWLock's read method
unsafe impl<T> Sync for RWLock<T> where T: Sync {}

pub struct ReadGuard<'a, T> {
    value: &'a T, // This needs something else...
    counter: &'a AtomicUsize
}

pub struct WriteGuard<'a, T> {
    value: &'a mut T,
    flag: &'a AtomicBool
}

impl<T> RWLock<T> {
    /// Create a new read-write lock for a value
    pub fn new(value: T) -> Self {
        let writing = AtomicBool::from(false);
        let readers = AtomicUsize::from(0);

        RWLock{ value: UnsafeCell::new(value), writing, readers }
    }

    /// Get a read lock
    pub fn read(&self) -> ReadGuard<'_, T> {
        while self.writing.load(Relaxed) {
            hint::spin_loop();
        }

        self.readers.fetch_add(1, Relaxed);

        let ptr = unsafe { self.value.get().as_ref_unchecked()};
        ReadGuard { value: ptr, counter: &self.readers}
    }

    /// Get a write lock
    pub fn write(&self) -> WriteGuard<'_, T> {
        // If we don't already have a writer
        loop {
            match self.writing.compare_exchange_weak(false, true, Relaxed, Relaxed) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }

        // If we have no readers
        while self.readers.load(Relaxed) > 0 {
            hint::spin_loop();
        }

        let ptr = unsafe { self.value.get().as_mut_unchecked() };
        WriteGuard { value: ptr, flag: &self.writing }
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

impl <'a, T> Deref for ReadGuard<'a, T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl <'a, T> Deref for WriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl <'a, T> DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl <'a, T> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        // SAFETY: `self` is pinned till after dropped.
        // unsafe { Drop::pin_drop(std::pin::Pin::new_unchecked(self)) }
        self.flag.store( false, Relaxed);
    }
}

impl <'a, T> Drop for ReadGuard<'a, T> {
    fn drop(&mut self) {
        // SAFETY: `self` is pinned till after dropped.
        // unsafe { Drop::pin_drop(std::pin::Pin::new_unchecked(self)) }
        self.counter.fetch_sub(1, Relaxed);
    }
}

#[test]
fn multiple_writer_block() {
    // Need to find a better way to do this.
    let rwlock = Arc::new(RWLock::new(1));
    let rw1 = rwlock.clone();
    let t1 = thread::spawn(move || {
        assert!(!rw1.writing.load(Relaxed));
        let mut item = rw1.write();
        assert!(rw1.writing.load(Relaxed) && item.flag.load(Relaxed));
        thread::sleep(Duration::from_millis(100));

        *item += 1;
    });

    // This SHOULD aquire a write lock after the first thread.
    let rw2 = rwlock.clone();
    let t2 = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        // Bit of a race condition here but this is what we want.
        assert!(rw2.writing.load(Relaxed));
        let mut item = rw2.write();
        assert!(item.flag.load(Relaxed));
        *item *= 2;
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let value = *rwlock.read();

    assert_eq!(value, 4);
}

#[test]
fn multiple_writers_blocking() {
    // This is an attempt to test some blocking action.
    let mut handles = vec![];
    let trigger = Arc::new(RWLock::new(false));
    let values : Arc<RWLock<Vec<usize>>> = Arc::new(RWLock::new(vec![]));
    for i in 0..20 {
        let t = trigger.clone();
        let v = values.clone();
        let h = thread::spawn(move || {
            // Should let us wait for everything to be created
            while !(t.read().value) {
                spin_loop();
            }

            v.write().push(i);
        });

        handles.push(h);
    }

    let v = values.clone();
    let t = trigger.clone();
    let whandle = thread::spawn(move || {
        while !(t.read().value) {
            spin_loop();
        }

        v.write().push(100);
    });

    handles.push(whandle);

    {
        let mut x = trigger.write();
        *x = true;
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let v = values.read().value;
    assert_eq!(values.read().len(), 21);
    assert!(v.contains(&100));

}
