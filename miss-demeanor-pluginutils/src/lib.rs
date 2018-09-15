extern crate hyper;
extern crate libc;
extern crate tokio;

use std::ffi::CStr;
use std::ptr;
use std::slice;

use hyper::{Body,Request};
use hyper::rt::Stream;
use tokio::runtime::Runtime;

#[no_mangle]
pub fn hyper_request_method(req: *const Request<Body>) -> *const libc::c_char {
    if let Some(r) = unsafe { req.as_ref() } {
        r.method().as_str().as_ptr() as *const libc::c_char
    } else {
        ptr::null()
    }
}

#[no_mangle]
pub fn hyper_request_body(req: *mut Request<Body>) -> *const u8 {
    let rt_result = Runtime::new();
    let mut rt = if let Ok(r) = rt_result {
        r
    } else {
        return ptr::null();
    };
    if let Some(r) = unsafe { req.as_mut() } {
        let result = rt.block_on(r.body_mut().by_ref().concat2());
        let chunk = if let Ok(c) = result {
            c
        } else {
            return ptr::null();
        };
        let bytes = &chunk;
        bytes.as_ptr()
    } else {
        ptr::null()
    }
}

#[no_mangle]
pub fn hyper_request_get_header(req: *mut Request<Body>, key: *const libc::c_char, str_len: usize)
        -> *const libc::c_char {
    let key_bytes = unsafe { slice::from_raw_parts(key as *const u8, str_len) };
    let key_str = if let Ok(s) = CStr::from_bytes_with_nul(key_bytes) {
        if let Ok(st) = s.to_str() {
            st
        } else {
            return ptr::null();
        }
    } else {
        return ptr::null();
    };
    if let Some(r) = unsafe { req.as_ref() } {
        if let Some(h) = r.headers().get(key_str) {
            if let Ok(s) = h.to_str() {
                s.as_ptr() as *const libc::c_char
            } else {
                ptr::null()
            }
        } else {
            ptr::null()
        }
    } else {
        ptr::null()
    }
}
