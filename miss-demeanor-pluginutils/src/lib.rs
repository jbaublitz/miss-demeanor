extern crate hyper;
extern crate libc;
extern crate tokio;

use std::{ptr,slice,str};

use hyper::{Body,Chunk,Request};
use hyper::rt::Stream;
use tokio::runtime::current_thread::block_on_all;

#[no_mangle]
pub unsafe extern "C" fn hyper_request_method(req: *const Request<Body>, len: *mut libc::c_int)
        -> *const libc::c_char {
    if let Some(r) = req.as_ref() {
        let s = r.method().as_str();
        *len = s.len() as libc::c_int;
        s.as_ptr() as *const libc::c_char
    } else {
        ptr::null()
    }
}

#[no_mangle]
pub unsafe extern "C" fn hyper_request_uri(req: *const Request<Body>, len: *mut libc::c_int)
        -> *const libc::c_char {
    if let Some(r) = req.as_ref() {
        let s = r.uri().to_string();
        *len = s.len() as libc::c_int;
        s.as_ptr() as *const libc::c_char
    } else {
        ptr::null()
    }
}

#[no_mangle]
pub unsafe extern "C" fn hyper_request_get_body(req: *mut Request<Body>, len: *mut libc::c_int)
        -> *const libc::c_void {
    if let Some(r) = req.as_mut() {
        let result = block_on_all(r.body_mut().by_ref().concat2());
        if let Ok(c) = result {
            *len = c.len() as libc::c_int;
            let chunk_ptr = c.as_ptr();
            std::mem::forget(chunk_ptr);
            chunk_ptr as *const libc::c_void
        } else {
            return ptr::null();
        }
    } else {
        ptr::null()
    }
}

#[no_mangle]
pub unsafe extern "C" fn hyper_request_free_body(chunk: *mut Chunk) {
    ptr::drop_in_place(chunk)
}

#[no_mangle]
pub unsafe extern "C" fn hyper_request_get_header(req: *mut Request<Body>, key: *const libc::c_char,
                                                  str_len: libc::c_int, value_len: *mut libc::c_int)
        -> *const libc::c_char {
    let key_bytes = slice::from_raw_parts(key as *const u8, str_len as usize);
    let key_str = if let Ok(s) = str::from_utf8(key_bytes) {
        s
    } else {
        return ptr::null();
    };

    if let Some(r) = req.as_ref() {
        if let Some(h) = r.headers().get(key_str) {
            if let Ok(s) = h.to_str() {
                *value_len = s.len() as libc::c_int;
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
