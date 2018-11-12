extern crate libc;

use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;

pub struct CRequest {
    pub method: CString,
    pub uri: CString,
    pub headers: HashMap<CString, CString>,
    pub body: CString,
}

#[allow(dead_code)]
#[no_mangle]
pub extern "C" fn request_get_method(req: *const CRequest) -> *const libc::c_char {
    let request = match unsafe { req.as_ref() } {
        Some(r) => r,
        None => {
            return ptr::null();
        },
    };
    request.method.as_ptr()
}

#[allow(dead_code)]
#[no_mangle]
pub extern "C" fn request_get_uri(req: *const CRequest) -> *const libc::c_char {
    let request = match unsafe { req.as_ref() } {
        Some(r) => r,
        None => {
            return ptr::null();
        },
    };
    request.uri.as_ptr()
}

#[allow(dead_code)]
#[no_mangle]
pub extern "C" fn request_get_header(req: *const CRequest, key: *mut libc::c_char) -> *const libc::c_char {
    let request = match unsafe { req.as_ref() } {
        Some(r) => r,
        None => {
            return ptr::null();
        },
    };
    let cstring = unsafe { CString::from_raw(key) };
    let header = match request.headers.get(&cstring) {
        Some(h) => h,
        None => {
            return ptr::null();
        },
    };
    header.as_ptr()
}

#[allow(dead_code)]
#[no_mangle]
pub extern "C" fn request_get_body(req: *const CRequest) -> *const libc::c_char {
    let request = match unsafe { req.as_ref() } {
        Some(r) => r,
        None => {
            return ptr::null();
        },
    };
    request.body.as_ptr()
}
