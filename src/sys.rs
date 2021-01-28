#[repr(C)]
pub struct ThreadHandle(::core::ffi::c_void);

#[repr(C)]
pub struct ThreadStartContext {
    pub th_stack: *mut ::core::ffi::c_void,
    pub th_internal: *mut ::core::ffi::c_void,
    pub th_start: ::core::option::Option<
        extern "C" fn(*mut ::core::ffi::c_void, *mut ThreadHandle, *mut ::core::ffi::c_void),
    >,
}

#[repr(C)]
pub struct Duration {
    pub seconds: i64,
    pub nanos: u32,
}

#[cfg(target_arch = "x86_64")]
pub type Result = i64;
#[cfg(all(target_arch = "x86", not(target_arch = "x86_64")))]
pub type Result = i32;

pub const THINVALID_HANDLE: Result = -0x100;
pub const THINTERRUPTED: Result = -0x101;
pub const THTIMEOUT: Result = -0x102;
pub const THKILLED: Result = -0x103;

#[allow(nonstandard_style)]
extern "C" {

    // ThreadHandle *GetCurrentThread(void);

    pub fn GetCurrentThread() -> *mut ThreadHandle;

    // _Noreturn void ExitThread(int);

    pub fn ExitThread(code: i32) -> !;

    // result StartThread(const ThreadStartContext *, ThreadHandle **);

    pub fn StartThread(ctx: *const ThreadStartContext, hdl: *mut *mut ThreadHandle) -> Result;

    // result ParkThread(void);

    pub fn ParkThread() -> Result;

    // result UnparkThread(ThreadHandle *);
    pub fn UnparkThread(hdl: *mut ThreadHandle) -> Result;

    // result AwaitAddress(void *);
    pub fn AwaitAddress(addr: *mut ::core::ffi::c_void) -> Result;

    // result SignalOne(void *);
    pub fn SignalOne(addr: *mut ::core::ffi::c_void) -> Result;
    // result SignalAll(void *);
    pub fn SignalAll(addr: *mut ::core::ffi::c_void) -> Result;

    // result SetBlockingTimeout(const duration *);

    pub fn SetBlockingTimeout(dur: *const Duration) -> Result;

    // result Sleep(const thread *);

    pub fn Sleep(dur: *const Duration) -> Result;

    // result InterruptThread(ThreadHandle *);

    pub fn InterruptThread(hdl: *mut ThreadHandle) -> Result;

    // result Interrupted(void);

    pub fn Interrupted() -> Result;

    // result JoinThread(ThreadHandle *);

    pub fn JoinThread(hdl: *mut ThreadHandle) -> Result;

    // result ClearBlockingTimeout()
    pub fn ClearBlockingTimeout() -> Result;
}
