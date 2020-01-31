package main

import "testing"

func HelpListen(t *testing.T) {
	l := Listen()
	c := l.Accept()

	buf := []byte("Hello")
	c.Read(buf)
	c.Write([]byte("Hello"))

	if string(buf) != "World" {
		t.Errorf("buf should be 'World', but it's %s", buf)
	}

	c.Read(buf)
	if string(buf) != "haha" {
		t.Errorf("buf should be 'haha', but it's %s", buf)
	}
}

func HelpConnect(t *testing.T) *Connection {
	c := Connect()

	buf := []byte("World")
	c.Write(buf)
	c.Read(buf)

	if string(buf) != "Hello" {
		t.Errorf("buf should be 'Hello', but it's %s", buf)
	}

	return c
}

func TestListenConnect(t *testing.T) {
	go HelpListen(t)
	c := HelpConnect(t)

	go func() { for { c.ChannelLoop() } }()

	ch := c.GetChannel(1) // 1 is the default used channel

	ch.Write([]byte("haha"))
}
