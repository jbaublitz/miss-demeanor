extern crate libc;
extern crate serde_json;

use std::collections::HashMap;
use std::error::Error;
use std::ffi::CString;
use std::ptr;
use std::str;

use serde_json::{Map,Value};

pub struct CRequest {
    pub method: CString,
    pub uri: CString,
    pub headers: HashMap<CString, CString>,
    pub body: CString,
}

impl CRequest {
    pub fn get_method(&self) -> Result<&str, str::Utf8Error> {
        self.method.to_str()
    }

    pub fn get_uri(&self) -> Result<&str, str::Utf8Error> {
        self.uri.to_str()
    }

    pub fn get_header(&self, key: &str) -> Option<&str> {
        let key_cstring = match CString::new(key.as_bytes()) {
            Ok(cs) => cs,
            Err(_) => return None,
        };
        self.headers.get(&key_cstring).and_then(|v| v.to_str().ok())
    }

    pub fn get_headers(&self) -> Result<String, Box<Error>> {
        let mut map = Map::<String, Value>::new();
        for (key, value) in self.headers.iter() {
            map.insert(key.to_str()?.to_string(), Value::from(value.to_str()?.to_string()));
        }
        Ok(serde_json::to_string(&map)?)
    }

    pub fn get_body(&self) -> Result<&str, str::Utf8Error> {
        self.body.to_str()
    }
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
