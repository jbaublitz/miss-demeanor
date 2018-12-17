extern crate libc;

use std::ffi::CStr;

extern "C" {
    fn request_get_method(request: *const libc::c_void) -> *const libc::c_char;
    fn request_get_uri(request: *const libc::c_void) -> *const libc::c_char;
    fn request_get_body(request: *const libc::c_void) -> *const libc::c_char;
}

pub fn trigger(request: *const libc::c_void) -> libc::c_int {
    let method = match unsafe { CStr::from_ptr(request_get_method(request)) }.to_str() {
        Ok(b) => b,
        Err(e) => {
            println!("{}", e);
            return 1;
        },
    };
    let uri = match unsafe { CStr::from_ptr(request_get_uri(request)) }.to_str() {
        Ok(b) => b,
        Err(e) => {
            println!("{}", e);
            return 1;
        },
    };
    let body = match unsafe { CStr::from_ptr(request_get_body(request)) }.to_str() {
        Ok(b) => b,
        Err(e) => {
            println!("{}", e);
            return 1;
        },
    };
	println!("{}", method);
	println!("{}", uri);
	println!("{}", body);
	0
}
