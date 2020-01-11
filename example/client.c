#include <stdio.h>
#include "mage.h"

int main() {
	void *addr = calloc(1, 0);
	void *len = calloc(1, 0);
	char buffer[1024] = { 0 };

	printf("abi_connect()\n");
	int sock = abi_connect(0, addr, len);

	printf("abi_send(%d, hello)\n", sock);
	abi_send(sock, "hello", 6, 0);

	printf("abi_recv(%d)\n", sock);
	abi_recv(sock, buffer, 28, 0);
	printf("%s\n", buffer);

	printf("bye");
	return 0;
}
