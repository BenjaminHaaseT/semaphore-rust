use atomic_wait::{wait, wake_one, wake_all};
use rand::{thread_rng, Rng};
use std::sync::atomic::{AtomicU32, Ordering::{Relaxed, Acquire, Release}};
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::thread;




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

unsafe impl Sync for Semaphore {}
unsafe impl Send for Semaphore {}

/// Basic implementation of a three state mutex.
pub struct Mutex<T> {
    semaphore: Semaphore,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    /// Associated method for creating a new `Mutex`.
    pub fn new(value: T) -> Self {
        Self {
            semaphore: Semaphore::init(0, 1),
            data: UnsafeCell::new(value),
        }
    }
    /// Method for locking the mutex. If the lock is unsuccessfully the current threads execution will
    /// block, and wait until it is woken up.
    pub fn lock(&self) -> MutexGuard<T> {
        // Once we return from `self.semaphore.signal()` we know the mutex is locked
        self.semaphore.signal();
        MutexGuard { mutex: self }
    }
}

unsafe impl<T> Sync for Mutex<T> where T: Send + Sync {}
unsafe impl<T> Send for Mutex<T> where T: Send + Sync {}

/// A guard for `Mutex<T>`. Ensures thread/memory safety of the data held by a `Mutex`
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // Safety: if we have a `MutexGuard` we know we have exclusive access to the data
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: if we have a `MutexGuard` we know we have exclusive access to the data
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // Reduce the count of the semaphore back to 0, unlocking the `Mutex`
        self.mutex.semaphore.wait();
        wake_one(&self.mutex.semaphore.counter);
    }
}



fn main() {
    use std::collections::VecDeque;
    let q = VecDeque::new();
    let mutex = &Mutex::new(q);

    thread::scope(|s| {
        for i in 0..10 {
            s.spawn(move || {
                let mut rng = rand::thread_rng();
                for _j in 0..100 {
                    let num = rng.gen_range(i * 10 ..= (i + 1) * 10);
                    mutex.lock().push_back(num);
                }
            });
        }
    });

    let mut counter = 0;
    while counter < 1000 {
        if let Some(generated_num) = mutex.lock().pop_front() {
            println!("generated_num: {generated_num}");
            counter += 1;
        }
    }

    println!("processed {counter} nums complete");
}
