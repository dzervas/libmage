package main

import (
	"fmt"
	"testing"
	"time"
)

func HelpListen(t *testing.T, finished chan bool) {
	fmt.Println("[Go] Listening")
	l := Listen("127.0.0.1:5555")
	fmt.Println("[Go] Accepting")

	seed := [32]byte{}
	for i := range seed {
		seed[i] = 2
	}

	c := l.Accept(seed, [32]byte{171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111})

	buf := []byte("Hello")
	fmt.Println("[Go] (L) Reading")
	c.Read(buf)
	fmt.Println("[Go] (L) Writing")
	c.Write(buf)

	if string(buf) != "World" {
		t.Errorf("buf should be 'World', but it's '%s'", buf)
	}

	finished <- true
}

func HelpConnect(t *testing.T) *Stream {
	fmt.Println("[Go] Connecting...")
	seed := [32]byte{}
	for i := range seed {
		seed[i] = 1
	}

	c := Connect("127.0.0.1:5555", seed, [32]byte{252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6})

	buf := []byte("World")
	fmt.Println("[Go] (C) Writing")
	c.Write(buf)
	fmt.Println("[Go] (C) Reading")
	c.Read(buf)

	if string(buf) != "World" {
		t.Errorf("buf should be 'World', but it's '%s'", buf)
	}

	return c
}

func TestListenConnect(t *testing.T) {
	listenFinish := make(chan bool)
	go HelpListen(t, listenFinish)
	time.Sleep(time.Second) // Wait for listener to start
	HelpConnect(t)

	<-listenFinish
}
