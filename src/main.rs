use atomic_wait::{wait, wake_one, wake_all};
use std::sync::atomic::{AtomicU32, Ordering::{Relaxed, Acquire, Release}};

/// A basic Semaphore implementation. Keeps track of a counter which can have configurable max and initial values.
/// Can be used to implement other synchronization primitives.
pub struct Semaphore {
    counter: AtomicU32,
    max: u32,
}

impl Semaphore {
    /// Associated function, initializes `self.max` to `u32::MAX` and `self.counter` to 0.
    pub fn new() -> Self {
        Self {
            counter: AtomicU32::new(0),
            max: u32::MAX,
        }
    }
    /// Method for configuring the initial value and max value of the `Semaphore`
    ///
    /// Panics: if `max` < `count`
    pub fn init(count: u32, max: u32) -> Self {
        assert!(count <= max, "count cannot be greater than max");
        Self {
            counter: AtomicU32::new(count),
            max,
        }
    }
    /// Increases the counter by 1 if possible. If the counter is strictly less than the maximum set
    /// then the method will increase the count, otherwise the method will block the current threads
    /// execution, waiting for the counter to be less than the maximum.
    pub fn signal(&self) {
        // Load the current value of `self.counter`
        // Acquire matches the Release ordering
        let mut cur_count = self.counter.load(Acquire);
        loop {
            // ensure cur_count is less than `self.max`, otherwise reload the count and try again
            if cur_count == self.max {
                wait(&self.counter, self.max);
                cur_count = self.counter.load(Acquire);
                continue;
            }
            // Attempt to increase the count by one
            match self.counter.compare_exchange(cur_count, cur_count + 1, Release, Relaxed) {
                Err(e) => cur_count = e,
                Ok(_) => return (),
            }
        }
    }
    /// Attempts to decrease the counter by 1 if possible. If the counter is equal to zero, then
    /// the method will block the current threads execution, waiting for the counter to be greater than zero.
    pub fn wait(&self) {
        // Load the current value of `self.counter`
        // Acquire matches Release ordering, ensures happens before relationship with any other threads altering `self.counter`
        let mut cur_count = self.counter.load(Acquire);
        loop {
            // ensure cur_count is greater than 0, otherwise reload the count and try again
            if cur_count == 0 {
                wait(&self.counter, 0);
                cur_count = self.counter.load(Acquire);
                continue;
            }
            // If we are successfully return from function, otherwise reset cur_count and try again
            match self.counter.compare_exchange(cur_count, cur_count - 1, Release, Relaxed) {
                Err(e) => cur_count = e,
                Ok(_) => return (),
            }
        }
    }
}

fn main() {
    println!("Hello, world!");
}
