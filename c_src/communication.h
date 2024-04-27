#include <sys/select.h>
#include <poll.h>
#include <sys/time.h>
#include <stdlib.h>
#include <mqueue.h>
#include <stdio.h>
#include <string.h>
#include "helper.h"
#include "rust_lib.h"

int send_msg(Message_Tag m, unsigned int attr);

uint64_t receive_msg();

struct timeval get_time();

struct timeval waiting();

struct timeval blocking();

void add_event(struct timeval);

void suppress_event(struct timeval);

int get_random();