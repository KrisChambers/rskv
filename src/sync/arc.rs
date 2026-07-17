#![allow(unused)]
use std::{
    ops::Deref,
    ptr::{NonNull, drop_in_place},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

struct ArcData<T> {
    ptr: NonNull<T>,
    count: NonNull<AtomicUsize>,
}

impl<T> ArcData<T> {
    pub fn new(value: T) -> Self {
        let ptr = NonNull::new(Box::into_raw(Box::new(value))).unwrap();
        let count = NonNull::new(Box::into_raw(Box::new(AtomicUsize::new(1)))).unwrap();

        Self { ptr, count }
    }

    fn get_count(&self) -> &AtomicUsize {
        unsafe { self.count.as_ref() }
    }

    fn get_value(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    fn inc(&self) {
        unsafe { self.get_count().fetch_add(1, Relaxed) };
    }

    fn dec(&self) {
        unsafe { self.get_count().fetch_sub(1, Relaxed) };
    }
}

impl<T> Clone for ArcData<T> {
    fn clone(&self) -> Self {
        self.inc();

        Self {
            ptr: self.ptr,
            count: self.count,
        }
    }
}

impl<T> Drop for ArcData<T> {
    fn drop(&mut self) {
        // SAFETY: `self` is pinned till after dropped.
        // unsafe { Drop::pin_drop(std::pin::Pin::new_unchecked(self)) }

        self.dec();

        if self.get_count().load(Relaxed) == 0 {
            unsafe { drop_in_place(self.ptr.as_ptr()) };
            unsafe { drop_in_place(self.count.as_ptr()) };
        }
    }
}

pub struct Arc<T> {
    data: ArcData<T>,
}

impl<T> Arc<T> {
    pub fn new(value: T) -> Self {
        Self {
            data: ArcData::new(value),
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.data.ptr.as_ref() }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        let data = self.data.clone();

        Self { data }
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
        assert_eq!(og.data.get_count().load(Relaxed), 10);

        let thing_addr = og.data.ptr.as_ptr() as usize;

        // They should all
        for clone in clones.iter() {
            // Have the same value
            assert_eq!(**clone, 1);

            // Have the same reference count
            assert_eq!(
                clone.data.get_count().load(Relaxed),
                og.data.get_count().load(Relaxed)
            );

            // Point to the same data as the original
            assert_eq!(thing_addr - clone.data.ptr.as_ptr() as usize, 0);
        }

        while let Some(clone) = clones.pop() {
            drop(clone);
            assert_eq!(og.data.get_count().load(Relaxed), clones.len() + 1);
        }

        assert_eq!(og.data.get_count().load(Relaxed), 1);
    }
}
