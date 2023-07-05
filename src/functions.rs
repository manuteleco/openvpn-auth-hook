//! Access to the original libc functions.
//!
//! The original libc functions are accessed via `dlsym` using the special
//! `RTLD_NEXT` pseudo handle. This handle allows us to access the next
//! occurrence of a function in the search order after the current library. This
//! is useful to avoid infinite recursion when we override a function that we
//! are also using.
//!
//! Each original libc function is exposed as Rust functions under the
//! `Functions` struct namespace, which is initialized lazily. They can be
//! invoked as `Functions::fopen` and so on.

use libc::{c_char, c_int, c_void, dlsym, FILE, RTLD_NEXT};
use once_cell::sync::Lazy;
use std::ffi::CString;
use std::mem;

static ORIGINAL_FUNCTIONS: Lazy<Functions> = Lazy::new(|| unsafe { Functions::new() });

type FOpenFn = extern "C" fn(filename: *const c_char, mode: *const c_char) -> *mut FILE;
type FGetsFn = extern "C" fn(buf: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char;
type FCloseFn = extern "C" fn(stream: *mut FILE) -> c_int;

/// Namespace for invoking the original libc functions.
pub struct Functions {
    fopen: FOpenFn,
    fgets: FGetsFn,
    fclose: FCloseFn,
}

impl Functions {
    unsafe fn new() -> Self {
        Functions {
            fopen: mem::transmute(Self::original_fn("fopen")),
            fgets: mem::transmute(Self::original_fn("fgets")),
            fclose: mem::transmute(Self::original_fn("fclose")),
        }
    }

    fn original_fn(fn_name: &str) -> *mut c_void {
        let open_name = CString::new(fn_name.as_bytes())
            // Safe to unwrap, as we know the string doesn't have any null bytes
            .unwrap();
        unsafe { dlsym(RTLD_NEXT, open_name.as_ptr()) }
    }

    pub fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE {
        (ORIGINAL_FUNCTIONS.fopen)(filename, mode)
    }

    pub fn fgets(buf: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char {
        (ORIGINAL_FUNCTIONS.fgets)(buf, n, stream)
    }

    pub fn fclose(stream: *mut FILE) -> c_int {
        (ORIGINAL_FUNCTIONS.fclose)(stream)
    }
}
