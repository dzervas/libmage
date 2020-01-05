#include <stdio.h>
#include "mage.h"

int main() {
	void *addr = calloc(1, 0);
	void *len = calloc(1, 0);
	char buffer[1024] = { 0 };

	printf("bind()\n");
	bind(0, addr, 0);

	printf("listen()\n");
	listen(0, 3);

	printf("accept()\n");
	accept(0, addr, len);

	printf("recv()\n");
	recv(0, buffer, 1024, 0);
	printf("%s\n", buffer);

	printf("send(world)\n");
	send(0, "world" , 6, 0);

	return 0;
}
