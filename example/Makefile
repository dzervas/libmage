CC=cc
CFLAGS=-lmage -L../target/debug/ -I../target/debug/

all: client.bin server.bin

fresh: clean all

libmage.so:
	cd ..
	cargo build --lib --manifest-path ../Cargo.toml

%.bin: %.c libmage.so
	$(CC) $(CFLAGS) -o $@ $<
	chmod u+x $@
	@echo -e "\e[92mTo run the binary 'LD_LIBRARY_PATH=../target/debug ./$@'\e[0m"

clean:
	rm -f client.bin server.bin mage.h
