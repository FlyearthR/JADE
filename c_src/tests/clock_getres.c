#include <time.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

int main()
{
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
        int ret1 = clock_getres(clocks[i], &tv1);

        struct timespec tv2;
        int ret2 = clock_getres(clocks[i], &tv2);

        if(ret1 != ret2) {
            printf("Error 1\n");
            exit(-1);
        }
        if(memcmp(&tv1, &tv2, sizeof(struct timespec))) {
            printf("Error 2\n");
            exit(-2);
        }
        printf("%i, %i\n", tv1.tv_sec, tv1.tv_nsec);
    }

    return 0;
}