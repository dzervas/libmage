#!/bin/sh
ln -s target/debug/libmage.so .
cbindgen -l c > example/mage.h
gcc -L. -lmage example/test.c -o test-c
LD_LIBRARY_PATH=. ./test-c
