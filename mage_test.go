package main

import (
	"fmt"
	"testing"
	"time"
)

func HelpListen(t *testing.T) {
	fmt.Println("[Go] Listening")
	l := Listen("127.0.0.1:5555")
	fmt.Println("[Go] Accepting")

	seed := [32]byte{}
	for i := range seed { seed[i] = 2 }

	c := l.Accept(seed, [32]byte{171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111})

	buf := []byte("Hello")
	c.Read(buf)
	c.Write(buf)

	if string(buf) != "World" {
		t.Errorf("buf should be 'World', but it's '%s'", buf)
	}

	fmt.Println("[Go] Channels:")

	c.Write([]byte("hoho!"))
	c.Read(buf)
	if string(buf) != "haha!" {
		t.Errorf("buf should be 'haha!', but it's '%s'", buf)
	}
}

func HelpConnect(t *testing.T) *Connection {
	fmt.Println("[Go] Connecting...")
	seed := [32]byte{}
	for i := range seed { seed[i] = 1 }

	c := Connect("127.0.0.1:5555", seed, [32]byte{252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6})

	buf := []byte("World")
	c.Write(buf)
	c.Read(buf)

	if string(buf) != "World" {
		t.Errorf("buf should be 'Hello', but it's '%s'", buf)
	}

	return c
}

func TestListenConnect(t *testing.T) {
	go HelpListen(t)
	time.Sleep(100 * time.Millisecond)
	c := HelpConnect(t)

	ch := c.GetChannel(0) // 0 is the default used channel
	buf := []byte("haha!")

	go func() {
		for {
			c.ChannelReadLoop()
		}
	}()

	go func() {
		for {
			c.ChannelWriteLoop()
		}
	}()

	//go func() {ch.Read(buf)}()
	fmt.Println("[Go] Loop")
	//c.ChannelLoop()
	fmt.Println("[Go] Read")
	ch.Read(buf)
	fmt.Println("[Go] Write")
	ch.Write(buf)

	if string(buf) != "hoho!" {
		t.Errorf("buf should be 'hoho!', but it's '%s'", buf)
	}
}
