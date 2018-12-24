# miss-demeanor
An audit compliance checker with a plugin interface

## Why miss-demeanor?
1. The name is clever and Missy Elliott is great.
2. miss-demeanor is heavily parallelized and as it is written
in Rust, the type system handles checking a lot of those
multithreading concerns for you. It handles
requests quickly, statelessly, and is specifically designed
to avoid deadlocks.
3. miss-demeanor is pluggable so you are the maker of your
own workflow here.
4. The plugin interface is flexible and dead simple.
5. See 1. It is still the most compelling reason.

## Building miss-demeanor
Install the Rust toolchain. Instructions can be found [here](https://rustup.rs/).

Navigate to `miss-demeanor/` and run `cargo build --release`.
Your executable will be located at `./target/release/miss-demeanor`.

## Using TLS with miss-demeanor
Currently the TLS library that miss-demeanor uses only supports a PCKS12/DER identity format.
This is not my choice and I hope to eventually be able to support PEM identites for the server.
That being said, there are test certs available for you to take a look at checked into the
project that I will ensure are up to date and can be used as a first step when evaluating
if miss-demeanor is the right solution for you.

The invocation is pretty simple: provide the path to `-f` for your PKCS12 identity file and use
the environment variable `PKCS12_PASSWORD` to supply the password.

## Config format
The config file is written in TOML.

Here is a sample config file with some comment explanations:

```
trigger_type = "c_abi" # Can also be "interpreted"

[server]
server_type = "webhook" # Can also be "unix_socket"
listen_addr = "127.0.0.1:8080" # Must be in the format IP:PORT
use_tls = false # You probably want this on unless you are running it over localhost - must pass -f on CLI when this is enabled

# One server endpoint
[[server.endpoints]]
path = "/pr" # URL path
trigger_name = "github-pr" # Unique name

# Another server endpoint
[[server.endpoints]]
path = "/merged"
trigger_name = "github-merged"

# Plugins
[[triggers]]
name = "github-merged" # Unique name
plugin_path = "./example-plugins/golang/github-merged.so" # Path to C ABI compatible shared object (.so)
```

The idea is to expose the server configuration declaratively.
The config file controls everything about the server -
endpoints, listen address, transport layer, plugins associated
with each endpoint, etc.

## Writing plugins
How do you actually write a plugin for miss-demeanor though?
First check out `miss-demeanor/example-plugins/` for code
examples.

The longer answer is this: a plugin can be one of two formats.

* It can be defined as any dynamic library (.so file on Linux for example)
that exports a C ABI compatible function symbol
(C ABI compatible simply means that it follows C calling
convention, etc. - that at a binary level it is
indistinguishable from C binaries) named `trigger`. To use this feature, set trigger type
to `c_abi`.

C function signature:

```
int trigger(void *http_request);
```

Rust function signature:

```
fn trigger(http_request: *const libc::c_void) -> libc::c_int;
```

C example:

```
#include "trigger.h"
#include <stdio.h>

int trigger(void *http_request) {
  printf("%s\n", request_get_method(http_request));
  return 0;
}
```

Golang example:

```
// #include "trigger.h"
// #cgo LDFLAGS: -lmissdemeanor -ldl
import "C"
import "unsafe"

import "fmt"

//export trigger
func trigger(http_request unsafe.Pointer) C.int {
  fmt.Println(C.GoString(C.request_get_method(http_request)))
}

func main {}
```

Rust example:

```
extern crate libc;

extern "C" {
    fn request_get_method(request: *const libc::c_void) -> *const libc::c_char;
}

#[no_mangle]
pub fn trigger(http_request: *const libc::c_void) -> libc::c_int {
    let method = match unsafe { CStr::from_ptr(request_get_method(request)) }.to_str() {
        Ok(b) => b,
        Err(e) => {
            println!("{}", e);
            return 1;
        },
    };
    println!("{}", method);
}
```

* It can be defined as an interpreted script with a shebang at the beginning. To use this feature,
set trigger type to `interpreted`.

Python example:

```
#!/usr/bin/python

import sys

method = sys.argv[1]

print(method)
```
