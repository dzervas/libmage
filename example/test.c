#include "mage.h"

int main() {
	void *addr = calloc(1, 0);
	void *len = calloc(1, 0);

	connect(0, addr, len);

	send(0, "hello", 6, 0);

	return 0;
}
