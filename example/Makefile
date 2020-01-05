CC=cc
CFLAGS=-lmage -L../target/debug/

all: client.bin server.bin

libmage.so:
	cd ..
	cargo build --lib --manifest-path ../Cargo.toml

mage.h:
	cbindgen -l c -o mage.h ../

%.bin: %.c mage.h
	$(CC) $(CFLAGS) -o $@ $<
	chmod u+x $@

clean:
	rm -f client.bin server.bin mage.h
