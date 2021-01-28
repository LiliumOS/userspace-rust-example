use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::sys;

pub struct SyncOnceCell<T> {
    init: AtomicBool,
    locked: AtomicBool,
    storage: UnsafeCell<MaybeUninit<T>>,
}

impl<T> SyncOnceCell<T> {
    pub const fn new() -> Self {
        Self {
            init: AtomicBool::new(false),
            locked: AtomicBool::new(false),
            storage: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn into_inner(self) -> Option<T> {
        if self.init.into_inner() {
            // SAFETY:
            // because init is set, this read will observe an initialized value
            Some(unsafe { self.storage.into_inner().assume_init() })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        if *self.init.get_mut() {
            // SAFETY:
            // Because init is set, storage holds an uninit value
            // Uniqueness of the storage is assured by the fact self is taken by &mut
            Some(unsafe { &mut *self.storage.get_mut().as_mut_ptr() })
        } else {
            None
        }
    }

    pub fn get(&self) -> Option<&T> {
        if self.init.load(Ordering::Relaxed) {
            Some(unsafe { &*(*self.storage.get()).as_ptr() })
        } else {
            None
        }
    }

    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        if self.init.load(Ordering::Acquire) {
            unsafe { &*(*self.storage.get()).as_ptr() }
        } else {
            while self.locked.swap(true, Ordering::Acquire) {
                // SAFETY:
                // AwaitAddress has no undefined behaviour
                unsafe {
                    sys::AwaitAddress(
                        self as *const _ as *const std::ffi::c_void as *mut std::ffi::c_void,
                    );
                }

                if self.init.load(Ordering::Acquire) {
                    return unsafe { &*(*self.storage.get()).as_ptr() };
                }
            }

            match catch_unwind(AssertUnwindSafe(f)) {
                Ok(v) => {
                    // SAFETY:
                    // No Data Race Occurs because of the efficient spinlock above
                    unsafe { (*self.storage.get()).as_mut_ptr().write(v) }
                    self.init.store(true, Ordering::Release);
                    self.locked.store(false, Ordering::Release);
                    unsafe {
                        sys::SignalAll(
                            self as *const _ as *const std::ffi::c_void as *mut std::ffi::c_void,
                        );
                    }
                    unsafe { &*(*self.storage.get()).as_ptr() }
                }
                Err(e) => {
                    self.locked.store(false, Ordering::Release);
                    unsafe {
                        sys::SignalOne(
                            self as *const _ as *const std::ffi::c_void as *mut std::ffi::c_void,
                        );
                    }
                    resume_unwind(e)
                }
            }
        }
    }
}

unsafe impl<T: Send + Sync> Sync for SyncOnceCell<T> {}
