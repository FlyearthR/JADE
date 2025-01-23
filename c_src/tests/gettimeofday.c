#include <sys/time.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

int main()
{
    struct timeval tv1;
    int ret1 = gettimeofday(&tv1, NULL);

    struct timeval tv2;
    int ret2 = gettimeofday(&tv2, NULL);

    if(ret1 != ret2) {
        printf("Error 1\n");
        exit(-1);
    }
    if(memcmp(&tv1, &tv2, sizeof(struct timeval))) {
        printf("Error 2\n");
        exit(-2);
    }

    printf("%i, %i", tv1.tv_sec, tv1.tv_usec);
    return 0;
}