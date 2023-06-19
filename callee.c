#include <mqueue.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

extern char **environ;
mqd_t fd = 0;
//#define FDI fd == 0 ? fd = atoi(getenv("FD")) : fd
#define FDI 4
#define FDO 3

int main(int argc, char **argv, char **envp)
{
	char msg[100];
	int ret = mq_receive(FDI, msg, sizeof(msg), NULL);
	if (ret == -1) {
		fprintf(stderr, "Error receiving message\n");
		exit(-6);
	}
	printf("%s\n", msg);
	snprintf(msg, 17, "Child %i is here", FDI);
	ret = mq_send(FDO, msg, strlen(msg), 1);

	return 0;
}
