#include <sys/select.h>
#include <poll.h>
#include <sys/time.h>
#include <time.h>

int before_timeval(struct timeval, struct timeval);

unsigned int timeval_to_uint_us(struct timeval);

struct timeval timespec_to_timeval(struct timespec);

struct timeval int_to_timeval_s(int);

struct timeval uint_to_timeval_us(unsigned int);

struct timeval add_timeval(struct timeval, struct timeval);
