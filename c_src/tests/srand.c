#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

int main()
{
    srand(42);
    int ret11 = rand();
    int ret12 = rand();

    srand(42);
    int ret21 = rand();
    int ret22 = rand();

    if(ret11 != ret21) {
        printf("Error 1: %i, %i\n", ret11, ret21);
        exit(-1);
    }
    if(ret12 != ret22) {
        printf("Error 2: %i, %i\n", ret12, ret22);
        exit(-1);
    }

    printf("%i, %i", ret11, ret12);
    return 0;
}