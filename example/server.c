#include <stdio.h>
#include "mage.h"

int main() {
	void *addr = calloc(1, 0);
	void *len = calloc(1, 0);
	char buffer[1024] = { 0 };

	/*printf("bind()\n");*/
	/*bind(0, addr, 0);*/

	printf("abi_listen()\n");
	int fd = abi_listen(0, 3);

	printf("abi_accept(%d)\n", fd);
	int sock = abi_accept(fd, addr, len);

	printf("abi_recv(%d)\n", sock);
	abi_recv(sock, buffer, 100, 0);
	printf("%s\n", buffer);

	printf("abi_send(%d, world)\n", sock);
	abi_send(sock, "world" , 6, 0);

	return 0;
}
