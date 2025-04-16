#include <sys/time.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>

#define DELAY_S 25
#define DELAY_US 15

struct timeval tv;

void handler(int signal) {
    struct timeval tv1;
    int ret1 = gettimeofday(&tv1, NULL);
    if (tv1.tv_sec != tv.tv_sec + DELAY_S || tv1.tv_usec != tv.tv_usec + DELAY_US) {
        printf("Error 1\n");
        exit(-1);
    }
    printf("woke up at: %i ; %i\n", tv1.tv_sec, tv1.tv_usec);
    exit(0);
}

int main()
{
    int ret1 = gettimeofday(&tv, NULL);

    struct sigaction sa;
	sa.sa_handler = handler;
	sa.sa_flags = SA_RESTART;
	sigemptyset(&sa.sa_mask);

	if (sigaction(SIGALRM, &sa, NULL) == -1) {
		perror("sigaction");
		return 1;
	}

    struct itimerval timer;
    timer.it_value.tv_sec = DELAY_S;
    timer.it_value.tv_usec = DELAY_US;
    timer.it_interval.tv_sec = 0;
    timer.it_interval.tv_usec = 0;
    if (setitimer(ITIMER_REAL, &timer, NULL) == -1) {
        perror("setitimer");
        return 1;
    }

    sleep(DELAY_S+5);
    return -2;
}