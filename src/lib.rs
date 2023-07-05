#![doc = include_str!("../README.md")]

mod functions;
mod params;
mod state;

use std::{
    ffi::{CStr, CString},
    ptr,
};

use functions::Functions;
use libc::{c_char, c_int, FILE};
use state::State;

/// The contents of the `auth-user-pass` file used by OpenVPN must be the
/// username and password, each in their own line. The first line must be the
/// username and the second line must be the password.
const PASSWORD_LINE_NUMBER: usize = 2;

/// Replacement for the `fopen` libc function.
///
/// If the file being opened is the `auth-user-pass` file, it tracks the
/// `FILE` pointer so that it can replace the password line when `fgets` is
/// called.
///
/// # Safety
///
/// `filename` and `mode` must be valid C strings.
///
#[no_mangle]
pub unsafe extern "C" fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE {
    let stream = Functions::fopen(filename, mode);
    if !stream.is_null() {
        if let Some(auth_file_path) = params::AUTH_FILE_PATH.as_ref() {
            let args = {
                (
                    CStr::from_ptr(filename).to_str(),
                    CStr::from_ptr(mode).to_str(),
                )
            };
            if args == (Ok(auth_file_path), Ok("r")) {
                State::add(stream);
            }
        }
    }
    stream
}

/// Replacement for the `fgets` libc function.
///
/// If the file being read is the `auth-user-pass` file, it replaces the
/// password line with the password stored in the binary.
///
/// # Safety
///
/// `buf` must be a valid C string. `stream` must be a valid pointer to a FILE,
/// created by `fopen` and not yet closed.
#[no_mangle]
pub unsafe extern "C" fn fgets(buf: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char {
    unsafe fn replace_line(buf: *mut c_char, n: c_int, new_line: &CString) {
        let new_line_len = new_line.as_bytes_with_nul().len();
        let available_space = n.try_into().unwrap_or(0);
        if new_line_len <= available_space {
            ptr::copy_nonoverlapping(new_line.as_ptr(), buf, new_line_len);
        } else {
            eprintln!(
                "[Hook] WARNING: Replacement line is too long to fit in the buffer \
                 ({new_line_len} > {available_space})"
            );
        }
    }

    let response_buffer = Functions::fgets(buf, n, stream);
    if !response_buffer.is_null() {
        // NOTE: The implementation here is quite simplistic, but good enough in
        // practice.
        //
        // Considering `fget`'s behavior (quoted excerpt from `man 3 fgets`):
        // > `fgets()` reads in at most one less than `size` characters from `stream`
        //   and stores them into the buffer pointed to by `s`. Reading stops after an
        //   EOF or a newline. If a newline is read, it is stored into the buffer. A
        //   terminating null byte ('\0') is stored after the last character in the
        //   buffer.
        //
        // Our assumption is that one call to `fgets` is equivalent to reading
        // one line of text. This is not necessarily true, as for lines longer
        // than the buffer size ([4096 as of OpenVPN
        // 2.6.5][openvpn-buffer-size]) `fgets` will only produce fractions of a
        // line. But it seems unlikely that this would be the case for the
        // `auth-user-pass` file, and OpenVPN itself also
        // [assumes][openvpn-auth-file-read] that username/password lines will
        // fit in the buffer.
        //
        // [openvpn-buffer-size]: https://github.com/OpenVPN/openvpn/blob/v2.6.5/src/openvpn/misc.h#L64-L73
        // [openvpn-auth-file-read]: https://github.com/OpenVPN/openvpn/blob/v2.6.5/src/openvpn/misc.c#L211-L252
        if State::inc_lines(stream) == Some(PASSWORD_LINE_NUMBER) {
            match params::password_line() {
                Ok(password_line) => replace_line(buf, n, &password_line),
                Err(err) => {
                    eprintln!("[Hook] ERROR: Unexpected error obtaining the password: {err}")
                }
            }
        }
    }
    response_buffer
}

/// Replacement for the `fclose` libc function.
///
/// If the file being closed is the `auth-user-pass` file, it removes the
/// `FILE` pointer from the list of tracked pointers.
///
/// # Safety
///
/// `stream` must be a valid pointer to a FILE, created by `fopen` and not yet
/// closed.
#[no_mangle]
pub unsafe extern "C" fn fclose(stream: *mut FILE) -> c_int {
    State::remove(stream);
    Functions::fclose(stream)
}
