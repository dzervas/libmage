package main

// #cgo LDFLAGS: -Ltarget/release -lmage
// #cgo CFLAGS: -Itarget/release
// #include "mage.h"
import "C"
import (
	"unsafe"
)

type Listener struct {
	index C.int
}

func Listen() *Listener {
	i := C.ffi_listen()
	return &Listener { index: i }
}

func (l *Listener) Accept() *Connection {
	i := C.ffi_accept(l.index)
	return &Connection { index: i }
}


type Connection struct {
	index C.int
}

func Connect() *Connection {
	i := C.ffi_connect()
	return &Connection { index: i }
}

func (c *Connection) GetChannel(i byte) *Channel {
	r := C.ffi_get_channel(c.index, C.uchar(i))
	return &Channel { index: r }
}

func (c *Connection) ChannelLoop() {
	C.ffi_channel_loop(c.index)
}

func (c *Connection) Read(buffer []byte) (int, error) {
	r := C.ffi_recv(c.index, unsafe.Pointer(&buffer[0]), C.ulong(len(buffer)))
	return int(r), nil
}

func (c *Connection) Write(buffer []byte) (int, error) {
	r := C.ffi_send(c.index, unsafe.Pointer(&buffer[0]), C.ulong(len(buffer)))
	return int(r), nil
}

type Channel struct {
	index C.int
}

func (c *Channel) Read(buffer []byte) (int, error) {
	r := C.ffi_recv_channel(c.index, unsafe.Pointer(&buffer[0]), C.ulong(len(buffer)))
	return int(r), nil
}

func (c *Channel) Write(buffer []byte) (int, error) {
	r := C.ffi_send_channel(c.index, unsafe.Pointer(&buffer[0]), C.ulong(len(buffer)))
	return int(r), nil
}

func main() {
	l := Listen()
	c := l.Accept()
	c.Write([]byte("hello"))
}