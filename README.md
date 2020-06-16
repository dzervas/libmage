[![Coverage Status](https://coveralls.io/repos/github/dzervas/mage/badge.svg?branch=master)](https://coveralls.io/github/dzervas/mage?branch=master)
![Test](https://github.com/dzervas/mage/workflows/Test/badge.svg)
![Go Test](https://github.com/dzervas/mage/workflows/Go%20Test/badge.svg)

# Mage

A tiny network protocol to be encapsulated in all kinds of transports.
Wrap a meterpreter with it and forget all your communication problems!

# Testing the Go binary

```shell script
cargo build --all-features --lib
LD_LIBRARY_PATH=target/debug go test
```
