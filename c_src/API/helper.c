#include "helper.h"

int before_timeval(struct timeval t1, struct timeval t2)
{
    return t1.tv_sec != t2.tv_sec ? t1.tv_sec < t2.tv_sec : t1.tv_usec < t2.tv_usec;
}

unsigned int timeval_to_uint_us(struct timeval t)
{
   return t.tv_sec*1000000+t.tv_usec;
}

struct timeval timespec_to_timeval(struct timespec t1)
{
    struct timeval t = {.tv_sec = t1.tv_sec, .tv_usec = t1.tv_nsec/1000};
    return t;
}

struct timeval int_to_timeval_s(int t)
{
    struct timeval ret = {.tv_sec = t, .tv_usec = 0};
    return ret;
}

struct timeval uint_to_timeval_us(unsigned int)
{
    
}

struct timeval add_timeval(struct timeval t1, struct timeval t2)
{
    struct timeval t = {.tv_sec = 0, .tv_usec = 0};
    return t;
}
