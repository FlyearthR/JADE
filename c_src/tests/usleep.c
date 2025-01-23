#include <sys/time.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(int argc, char** argv)
{
    int delay = argc > 1 ? atoi(argv[1]) : 2;

    struct timeval tv1;
    int ret1 = gettimeofday(&tv1, NULL);

    usleep(delay);

    struct timeval tv2;
    int ret2 = gettimeofday(&tv2, NULL);

    if (ret1 != ret2)
    {
        printf("Error 1\n");
        exit(-1);
    }
    if (tv1.tv_sec != tv2.tv_sec)
    {
        printf("Error 2\n");
        exit(-2);
    }
    if (tv1.tv_usec + delay != tv2.tv_usec)
    {
        printf("Error 3\n");
        exit(-3);
    }

    printf("%i, %i\n", tv1.tv_sec, tv1.tv_usec);
    printf("%i, %i\n", tv2.tv_sec, tv2.tv_usec);
    return 0;
}