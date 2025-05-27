#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>

int main()
{
    int ret1;
    getentropy(&ret1, sizeof(int));

    printf("%i", ret1);
    return 0;
}