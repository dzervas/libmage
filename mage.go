package main

// `cargo build --release --features ffi` to use this!
// On Linux LD_LIBRARY_PATH=target/release env is required!

// #cgo LDFLAGS: -Ltarget/debug -lmage
// #cgo CFLAGS: -Itarget/debug
// #include "target/debug/mage.h"
import "C"

import (
	"fmt"
	"unsafe"
)

type Listener struct {
	index C.ulong
}

func Listen(addr string) *Listener {
	addrPtr := C.CString(addr)
	addrCharPtr := (*C.schar)(addrPtr)

	i := C.ffi_listen_opt(addrCharPtr)

	C.free(unsafe.Pointer(addrPtr))

	return &Listener{index: i}
}

func (l *Listener) Accept(seed [32]byte, key [32]byte) *Stream {
	seedPtr := unsafe.Pointer(&seed[0])
	seedCharPtr := (*C.uchar)(seedPtr)
	keyPtr := unsafe.Pointer(&key[0])
	keyCharPtr := (*C.uchar)(keyPtr)

	i := C.ffi_accept_opt(l.index, 1, seedCharPtr, keyCharPtr)
	fmt.Printf("[Go] New accept: %d\n", uint64(i))

	return &Stream{index: i}
}

type Stream struct {
	index C.ulong
}

func Connect(addr string, seed [32]byte, key [32]byte) *Stream {
	addrPtr := C.CString(addr)
	addrCharPtr := (*C.schar)(addrPtr)
	seedPtr := unsafe.Pointer(&seed[0])
	seedCharPtr := (*C.uchar)(seedPtr)
	keyPtr := unsafe.Pointer(&key[0])
	keyCharPtr := (*C.uchar)(keyPtr)

	i := C.ffi_connect_opt(addrCharPtr, 0, seedCharPtr, keyCharPtr)
	fmt.Printf("[Go] New connect: %d\n", uint64(i))

	C.free(unsafe.Pointer(addrPtr))

	return &Stream{index: i}
}

func (c *Stream) Read(buffer []byte) (int, error) {
	bufferLen := C.ulong(len(buffer))
	bufferPtr := unsafe.Pointer(&buffer[0])

	r := C.ffi_recv(c.index, bufferPtr, bufferLen)

	return int(r), nil
}

func (c *Stream) Write(buffer []byte) (int, error) {
	bufferLen := C.ulong(len(buffer))
	bufferPtr := unsafe.Pointer(&buffer[0])

	r := C.ffi_send(c.index, bufferPtr, bufferLen)

	return int(r), nil
}

func main() {
}
