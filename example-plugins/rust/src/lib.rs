use missdemeanor::CRequest;

#[no_mangle]
pub fn trigger(request: *const CRequest) -> libc::c_int {
    let req = match unsafe { request.as_ref() } {
        Some(r) => r,
        None => return 1,
    };
    let method = match req.get_method() {
        Ok(b) => b,
        Err(e) => {
            println!("{}", e);
            return 1;
        },
    };
    let uri = match req.get_uri() {
        Ok(b) => b,
        Err(e) => {
            println!("{}", e);
            return 1;
        },
    };
    let body = match req.get_body() {
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
