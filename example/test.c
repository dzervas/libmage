#include "mage.h"

int main() {
	void *addr = calloc(1, 0);
	void *len = calloc(1, 0);

	printf("connect()\n");
	connect(0, addr, len);

	printf("send(hello)\n");
	send(0, "hello", 6, 0);

	printf("bye");
	return 0;
}
