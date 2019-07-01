#![deny(missing_docs)]
//! An append-only, concurrent arena

use std::cell::UnsafeCell;
use std::mem;

use arrayvec::ArrayVec;
use parking_lot::Mutex;

const N_LANES: usize = 64;
const USIZE_BITS: usize = mem::size_of::<usize>() * 8;

#[inline(always)]
fn lane_size(n: usize) -> usize {
    2_usize.pow(n as u32) * 2
}

#[inline(always)]
fn lane_offset(offset: usize) -> (usize, usize) {
    let i = offset / 2 + 1;
    let lane = USIZE_BITS - i.leading_zeros() as usize - 1;
    let offset = offset - (2usize.pow(lane as u32) - 1) * 2;
    (lane, offset)
}

impl<T> Default for Bunch<T> {
    fn default() -> Self {
        Bunch {
            lanes: Default::default(),
            len: Default::default(),
        }
    }
}

unsafe impl<T> Send for Bunch<T> {}
unsafe impl<T> Sync for Bunch<T> {}

/// The main Arena type
pub struct Bunch<T> {
    lanes: UnsafeCell<ArrayVec<[Vec<T>; N_LANES]>>,
    len: Mutex<usize>,
}

impl<T> Bunch<T> {
    /// Creates a new arena
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes to the arena, returning a reference to the pushed element
    pub fn push(&self, t: T) -> &T {
        let len = &mut *self.len.lock();
        let (lane, offset) = lane_offset(*len);

        if offset == 0 {
            unsafe {
                let lanes = &mut *self.lanes.get();
                let size = lane_size(lane);
                lanes.push(Vec::with_capacity(size));
            }
        }

        unsafe {
            let lanes = &mut *self.lanes.get();
            lanes[lane].push(t);
            *len += 1;
            lanes[lane].get(offset).expect("just pushed")
        }
    }

    /// Gets a reference into the arena
    pub fn get(&self, idx: usize) -> &T {
        let (lane, offset) = lane_offset(idx);
        unsafe { &(*self.lanes.get())[lane][offset] }
    }

    /// Returns the number of elements in the Bunch
    pub fn len(&self) -> usize {
        *self.len.lock()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::sync::Arc;

    #[test]
    fn it_works() {
        let p = Bunch::new();

        assert_eq!(p.push(3), &3);

        assert_eq!(p.get(0), &3);
    }

    #[test]
    fn multiple() {
        let p = Bunch::new();

        for i in 0..10_000 {
            let r = p.push(i);
            assert_eq!(r, &i);
        }

        for i in 0..10_000 {
            assert_eq!(p.get(i), &i);
        }
    }

    #[test]
    fn multithreading() {
        let vec = Arc::new(Bunch::new());
        let n = 100_000;

        let n_threads = 16;

        let mut handles = vec![];

        for t in 0..n_threads {
            let vec = vec.clone();
            handles.push(std::thread::spawn(move || {
                for i in 0..n {
                    if i % n_threads == t {
                        vec.push(i);
                    }
                }
            }))
        }

        for h in handles {
            h.join().unwrap();
        }

        let mut result = vec![];

        for i in 0..n {
            result.push(vec.get(i));
        }

        result.sort();

        for i in 0..n {
            assert_eq!(result[i], &i)
        }
    }

    #[test]
    fn dropping() {
        let n = 100_000;
        let n_threads = 16;

        let mut arcs = vec![];

        for i in 0..n {
            arcs.push(Arc::new(i));
        }

        for i in 0..n {
            assert_eq!(Arc::strong_count(&arcs[i]), 1);
        }

        let wrapped_arcs = Arc::new(arcs);

        {
            let vec = Arc::new(Bunch::new());

            let mut handles = vec![];

            for t in 0..n_threads {
                let vec = vec.clone();
                let wrapped_clone = wrapped_arcs.clone();
                handles.push(std::thread::spawn(move || {
                    for i in 0..n {
                        if i % n_threads == t {
                            vec.push(wrapped_clone[i].clone());
                        }
                    }
                }))
            }

            for h in handles {
                h.join().unwrap();
            }

            for i in 0..n {
                assert_eq!(Arc::strong_count(&wrapped_arcs[i]), 2);
            }
        }
        // Bunch dropped here
        for i in 0..n {
            assert_eq!(Arc::strong_count(&wrapped_arcs[i]), 1);
        }
    }

    #[test]
    #[should_panic]
    fn out_of_bounds_access() {
        let bunch = Bunch::new();
        bunch.push("hello");

        bunch.get(1);
    }
}
