#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>

int main()
{
    int fd = open("/dev/random", O_RDONLY | O_CLOEXEC);
    if (fd == -1)
        exit(-1);
    int ret1;
    if (read(fd, &ret1, sizeof(int)) != sizeof(int))
        exit(-2);
    printf("%i", ret1);
    return 0;
}