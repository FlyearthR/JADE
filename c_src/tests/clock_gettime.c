#include <time.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(int argc, char** argv)
{
    int delay =  2;

    clockid_t clocks[] = {
        CLOCK_REALTIME,
        CLOCK_REALTIME_ALARM,
        CLOCK_REALTIME_COARSE,
        CLOCK_TAI,
        CLOCK_MONOTONIC,
        CLOCK_MONOTONIC_COARSE,
        CLOCK_MONOTONIC_RAW,
        CLOCK_BOOTTIME,
        CLOCK_BOOTTIME_ALARM
    };

    for (int i = 0 ; i < sizeof(clocks)/sizeof(clockid_t) ; i++) {
        struct timespec tv1;
        int ret1 = clock_gettime(clocks[i], &tv1);

        sleep(delay);

        struct timespec tv2;
        int ret2 = clock_gettime(clocks[i], &tv2);

        if(ret1 != ret2) {
            printf("Error 1\n");
            exit(-1);
        }
        if (tv1.tv_sec + delay != tv2.tv_sec)
        {
            printf("Error 2: (%i, %i) != (%i, %i)\n", tv1.tv_sec, tv1.tv_nsec, tv2.tv_sec, tv2.tv_nsec);
            exit(-2);
        }
        if (tv1.tv_nsec != tv2.tv_nsec)
        {
            printf("Error 3\n");
            exit(-3);
        }
        printf("%i, %i\n", tv1.tv_sec, tv1.tv_nsec);
    }    

    return 0;
}