#include <fcntl.h>
#include <sys/stat.h>
#include <mqueue.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

#define NB_FOLLOWER 3

int main()
{
	mqd_t qs[NB_FOLLOWER+1];
	char name[11];
	int ret;
	int failed = 0;
	struct mq_attr attr = {0, NB_FOLLOWER+1, 100, 0};
	for (int i = 0 ; i <= NB_FOLLOWER ; i++) {
	    snprintf(name, 11, "/nts_mq_%i", i);
	    qs[i] = mq_open(name, O_RDWR | O_CREAT, 660, &attr);
	    if (qs[i] == -1) {
		fprintf(stderr, "Error opening queue %i\n", i);
		perror(NULL);
		exit(-1);
	    }
	    ret = fcntl(qs[i], F_SETFD, O_CLOEXEC, 0);
	    if (ret == -1) {
		fprintf(stderr, "Error changing close-on-exec flag on queue %i\n", i);
		exit(-2);
	    }
	}
	char* msg = "Are you born my child?\n";
	for (int i = 0 ; i < NB_FOLLOWER ; i++) {
	    ret = fork();
	    if (ret == -1) {
		fprintf(stderr, "Error forking process\n");
		exit(-3);
	    }
	    if (ret == 0) {
		char* const argv[] = {"./callee", NULL};
        	char* envp[2];
	        char fd[9];
	       	envp[0] = fd;
		envp[1] = NULL;
		snprintf(envp[0], 6, "FD=%i", qs[i+1]);
		ret = execve("./callee", argv, envp);
		if (ret == -1) {
		    fprintf(stderr, "Error executing callee\n");
		    exit(-4);
		}
	    }
	    /*ret = mq_send(qs[i+1], msg, strlen(msg), 1);
	    if (ret == -1) {
		fprintf(stderr, "Error sending message\n");
		exit(-5);
	    }*/
	}
	/* main_loop */
	/*char rep[100];
	for (int i = 0 ; i < NB_FOLLOWER ; i++) {
	    ret = mq_receive(qs[0], rep, sizeof(rep), NULL);
	    if (ret == -1) {
		fprintf(stderr, "Error receiving message on main channel\n");
	        exit(-8);
	    }
	    printf("%s\n", rep);
	}*/
	for (int i = 0 ; i <= NB_FOLLOWER ; i++) {
	    snprintf(name, 11, "/nts_mq_%i", i);
	    ret = mq_unlink(name);
	    if (ret == -1) {
		fprintf(stderr, "Error unlinking queue %i", i);
		failed = 1;
	    }
	}
	if (failed)
	    exit(-6);
	return 0;
}
