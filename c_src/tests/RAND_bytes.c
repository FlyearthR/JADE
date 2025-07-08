#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <openssl/rand.h>

int main()
{    
    int ret1;
    RAND_bytes((unsigned char *) &ret1, sizeof(int));

    printf("%i", ret1);
    return 0;
}