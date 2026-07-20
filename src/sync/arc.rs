#![allow(unused)]
use std::{
    ops::Deref,
    ptr::{NonNull, drop_in_place},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

struct ArcData<T> {
    value: T,
    count: AtomicUsize,
}

/// The inner data of an Arc that will be stored together on the heap
impl<T> ArcData<T> {
    pub fn new(value: T) -> Self {
        let count = AtomicUsize::new(1);

        Self { value, count }
    }
}

/// A custom Arc implementation using pointers.
pub struct Arc<T> {
    data: NonNull<ArcData<T>>,
}

impl <T> Drop for Arc<T> {
    fn drop(&mut self) {
        // SAFETY: `self` is pinned till after dropped.
        //unsafe { Drop::pin_drop(std::pin::Pin::new_unchecked(self)) }

        self.dec();

        if self.get_count() == 0 {
            unsafe { drop_in_place(self.data.as_ptr()) };
        }
    }
}

impl<T> Arc<T> {
    pub fn new(value: T) -> Self {
        let inner = Box::new(ArcData::new(value));
        let data = NonNull::new(Box::into_raw(inner)).unwrap();
        Self {
            data,
        }
    }

    fn get_data(&self) -> &ArcData<T> {
        unsafe { self.data.as_ref() }
    }

    fn inc(&self) {
        let data = self.get_data();
        data.count.fetch_add(1, Relaxed);
    }

    fn dec(&self) {
        let data = self.get_data();
        data.count.fetch_sub(1, Relaxed);
    }

    fn get_count(&self) -> usize {
        let data = self.get_data();
        data.count.load(Relaxed)
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.data.as_ref().value }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        self.inc();

        Self { data: self.data }
    }
}

// Need to think about this a bit more
// T: Send -> Safe to move T across thread boundaries (transfer of ownership between threads).
// T: Sync -> Safe to share a &T between threads.
unsafe impl<T> Send for Arc<T> where T: Send {}
unsafe impl<T> Sync for Arc<T> where T: Sync {}

// Helpful :: "T : Sync if and only if &T : Send"

#[cfg(test)]
mod test {
    use super::*;
    struct Test {
        value: &'static str,
    }

    #[test]
    fn basic_arc_deref() {
        let thing = Arc::new(Test { value: "boop" });

        assert_eq!(thing.value, "boop");
    }

    #[test]
    fn same_data() {
        let mut clones = vec![];
        let og = Arc::new(1);

        for _ in 0..9 {
            clones.push(og.clone());
        }

        // There should be 10 references
        assert_eq!(og.get_count(), 10);

        let thing_addr = og.data.as_ptr() as usize;

        // They should all
        for clone in clones.iter() {
            // Have the same value
            assert_eq!(**clone, 1);

            // Have the same reference count
            assert_eq!(
                clone.get_count(),
                og.get_count()
            );

            // Point to the same data as the original
            assert_eq!(thing_addr - clone.data.as_ptr() as usize, 0);
        }

        while let Some(clone) = clones.pop() {
            drop(clone);
            assert_eq!(og.get_count(), clones.len() + 1);
        }

        assert_eq!(og.get_count(), 1);
    }
}
