package main

// #include "trigger.h"
// #cgo LDFLAGS: -L. -lmissdemeanor -ldl
import "C"

import (
  "fmt"
  "unsafe"
)

//export trigger
func trigger(request unsafe.Pointer) C.int {
  method := C.GoString(C.request_get_method(request))
  uri := C.GoString(C.request_get_uri(request))
  body := C.GoString(C.request_get_body(request))
  fmt.Println(method)
  fmt.Println(uri)
  fmt.Println(body)
  return 0
}

func main() {}
