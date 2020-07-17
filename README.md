[![Coverage Status](https://coveralls.io/repos/github/dzervas/libmage/badge.svg?branch=master)](https://coveralls.io/github/dzervas/libmage?branch=master)
[![Test](https://github.com/dzervas/libmage/workflows/Test/badge.svg)](https://github.com/dzervas/libmage/actions?query=workflow%3ATest)
[![Go Test](https://github.com/dzervas/libmage/workflows/Go%20Test/badge.svg)](https://github.com/dzervas/libmage/actions?query=workflow%3A%22Go+Test%22)

# Mage

A tiny network protocol to be encapsulated in all kinds of transports.
Wrap a meterpreter with it and forget all your communication problems!

# Testing the Go binary

```shell script
cargo build --all-features --lib
LD_LIBRARY_PATH=target/debug go test
```
