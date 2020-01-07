#include <stdio.h>
#include "mage.h"

int main() {
	void *addr = calloc(1, 0);
	void *len = calloc(1, 0);
	char buffer[1024] = { 0 };

	/*printf("bind()\n");*/
	/*bind(0, addr, 0);*/

	printf("listen()\n");
	int fd = listen(0, 3);

	printf("accept()\n");
	int sock = accept(fd, addr, len);

	printf("recv()\n");
	recv(sock, buffer, 1024, 0);
	printf("%s\n", buffer);

	printf("send(world)\n");
	send(sock, "world" , 6, 0);

	return 0;
}
