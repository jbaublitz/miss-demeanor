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
own workflow here. Whatever you can do in a language that can
export C ABI compatible function symbols, you can do in a
miss-demeanor plugin. More simply, this means that C,
Rust and Golang are all supported languages for plugins.
4. The plugin interface is flexible and dead simple. Define
a function `trigger` that takes a pointer to an HTTP request
and returns an integer and build anything else you need around
it.
5. See 1. It is still the most compelling reason.

## Config format
The config file is written in TOML.

Here is a sample config file with some comment explanations:

```
[server]
server_type = "webhook" # Can also be "unix_socket"
listen_addr = "127.0.0.1:8080" # Must be in the format IP:PORT
use_tls = false # You probably want this on unless you are running it over localhost - must pass -p and -f on CLI when this is enabled

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

The longer answer is this: a plugin for miss-demeanor is
defined as any dynamic library (.so file on Linux for example)
that exports a C ABI compatible function symbol
(C ABI compatible simply means that it follows C calling
convention, etc. - that at a binary level it is
indistinguishable from C binaries) named `trigger`
with the C signature:

```
int trigger(void *http_request);
```

or Rust signature:

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
// #cgo LDFLAGS: -lmissdemeanor
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

#[no_mangle]
pub fn trigger(http_request: *const libc::c_void) -> libc::c_int {
  println!("{}", request_get_method(http_request));
}
```
