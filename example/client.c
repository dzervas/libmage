#include <stdio.h>
#include "mage.h"

int main() {
	void *addr = calloc(1, 0);
	void *len = calloc(1, 0);
	char buffer[1024] = { 0 };

	printf("connect()\n");
	int sock = connect(0, addr, len);

	printf("send(hello)\n");
	send(sock, "hello", 6, 0);

	printf("recv()\n");
	recv(sock, buffer, 1024, 0);
	printf("%s\n", buffer);

	printf("bye");
	return 0;
}
