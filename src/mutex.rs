use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    panic::RefUnwindSafe,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

use crate::sys::{self, GetCurrentThread, ThreadHandle};

struct RawMutex {
    owner: AtomicPtr<ThreadHandle>,
}

impl RawMutex {
    pub const fn new() -> Self {
        Self {
            owner: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    pub fn lock(&self) {
        // SAFETY:
        // GetCurrentThread prescribes no undefined behaviour
        let hdl = unsafe { GetCurrentThread() };
        loop {
            let owner = self.owner.load(Ordering::Relaxed);
            if owner == hdl {
                panic!("Deadlock ahead (not a recursive mutex)");
            }
            if owner.is_null() {
                if let Ok(_) =
                    self.owner
                        .compare_exchange(owner, hdl, Ordering::Acquire, Ordering::Relaxed)
                {
                    return;
                }
            }

            // SAFETY:
            // AwaitAddress has no undefined behaviour
            unsafe {
                sys::AwaitAddress(
                    self as *const _ as *const std::ffi::c_void as *mut std::ffi::c_void,
                );
            }
        }
    }

    pub unsafe fn unlock(&self) {
        self.owner.store(core::ptr::null_mut(), Ordering::Release);
        #[allow(unused_unsafe)]
        unsafe {
            sys::SignalOne(self as *const _ as *const std::ffi::c_void as *mut std::ffi::c_void);
        }
    }
}

unsafe impl Send for RawMutex {}
unsafe impl Sync for RawMutex {}

pub struct Mutex<T: ?Sized> {
    raw: RawMutex,
    poisoned: AtomicBool,
    cell: UnsafeCell<T>,
}

unsafe impl<T: Sync + Send + ?Sized> Sync for Mutex<T> {}

impl<T: ?Sized> RefUnwindSafe for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(x: T) -> Self {
        Self {
            raw: RawMutex::new(),
            poisoned: AtomicBool::new(false),
            cell: UnsafeCell::new(x),
        }
    }

    pub fn into_inner(self) -> T {
        self.cell.into_inner()
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn get_mut(&mut self) -> &mut T {
        self.cell.get_mut()
    }

    pub fn lock(&self) -> Result<MutexGuard<T>> {
        self.raw.lock();
        let guard = MutexGuard { inner: self };
        if self.poisoned.load(Ordering::Release) {
            Err(PoisonError(guard))
        } else {
            Ok(guard)
        }
    }
}

type Result<T> = std::result::Result<T, PoisonError<T>>;

pub struct PoisonError<T>(T);

impl<T> PoisonError<T> {
    pub fn into_inner(self) -> T {
        self.0
    }

    pub fn get(&self) -> &T {
        &self.0
    }
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

pub struct MutexGuard<'a, T: ?Sized> {
    inner: &'a Mutex<T>,
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.inner.poisoned.store(true, Ordering::Release)
        }
        // SAFETY:
        // The lock is acquired
        unsafe { self.inner.raw.unlock() }
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY:
        // The lock is acquired, we have shared access to the inner field
        unsafe { &*self.inner.cell.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner.cell.get() }
    }
}
