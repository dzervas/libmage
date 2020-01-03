#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

int connect(int _socket, const void *_sockaddr, void *_address_len);

ssize_t recv(int _socket, void *msg, uintptr_t size, int _flags);

ssize_t send(int _socket, const void *msg, uintptr_t size, int _flags);
