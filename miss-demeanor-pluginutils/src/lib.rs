extern crate hyper;
extern crate libc;
extern crate tokio;

use std::ffi::CStr;
use std::ptr;
use std::slice;

use hyper::{Body,Request,Response,StatusCode};
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

#[no_mangle]
pub fn hyper_response_code(req: *mut Response<Body>, status: u16) -> libc::c_int {
    if let Some(r) = unsafe { req.as_mut() } {
        *r.status_mut() = if let Ok(sc) = StatusCode::from_u16(status) {
            sc
        } else {
            return -1;
        };
        0
    } else {
        -1
    }
}

#[no_mangle]
pub fn hyper_response_add_header(req: *mut Response<Body>, key: *const libc::c_char, key_len: usize,
                                 value: *const libc::c_char, value_len: usize) -> libc::c_int {
    let key_bytes = unsafe { slice::from_raw_parts(key as *const u8, key_len) };
    let value_bytes = unsafe { slice::from_raw_parts(value as *const u8, value_len) };
    let (key_str, value_str) = if let (Ok(k), Ok(v)) = (CStr::from_bytes_with_nul(key_bytes),
            CStr::from_bytes_with_nul(value_bytes)) {
        if let (Ok(key_st), Ok(value_st)) = (k.to_str(), v.to_str()) {
            (key_st, value_st)
        } else {
            return -1;
        }
    } else {
        return -1;
    };
    let header_value = if let Ok(v) = value_str.parse() {
        v
    } else {
        return -1;
    };
    if let Some(r) = unsafe { req.as_mut() } {
        r.headers_mut().insert(key_str, header_value);
        0
    } else {
        -1
    }
}

#[no_mangle]
pub fn hyper_response_body(req: *mut Response<Body>, body: *const u8, body_len: usize) -> libc::c_int {
    let body_bytes = unsafe { slice::from_raw_parts(body, body_len) };
    if let Some(r) = unsafe { req.as_mut() } {
        *r.body_mut() = Body::from(body_bytes);
        0
    } else {
        -1
    }
}
