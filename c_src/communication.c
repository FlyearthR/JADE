#include "communication.h"

int id = 0;
#define ID id

#define FDI 4
#define FDO 3

void __attribute__((constructor)) init_fd() {
    //printf("before loading and freopen\n");
    id = atoi(getenv("ID"));
    char out[10] = "";
    strcat(out, getenv("ID"));
    strcat(out, ".out");
    freopen(out, "w", stdout);
    char err[10] = "";
    strcat(err, getenv("ID"));
    strcat(err, ".err");
    freopen(err, "w", stderr);
    //printf("before loading\n");
    fflush(stdout);
}

int send_msg(Message_Tag m, unsigned int attr)
{
    //printf("\tsend_msg beginning\n");
    fflush(stdout);
    Message msg = {.tag=m, .get_time=ID};
    if (m == AddStep || m == DelStep) {
        msg.add_step._1 = attr;
    }
    Buffer b = serialize(msg);
    int ret = mq_send(FDO, b.buffer, 10, (m == GetTime || m == GetRand) ? 2 : 1);
    //printf("\tsend_msg end\n");
    fflush(stdout);
    return ret;
}

uint64_t receive_msg()
{
    Buffer msg;
    //printf("\treceive_msg beginning\n");
    fflush(stdout);
    int ret = mq_receive(FDI, msg.buffer, 10, NULL);
	if (ret == -1) {
		fprintf(stderr, "Error receiving message\n");
		perror("Error: ");
		exit(-6);
	}
    //printf("\treceive_msg end\n");
    fflush(stdout);
    return deserialize(msg).wake_up;
}

struct timeval get_time()
{
    //printf("get_time beginning\n");
    fflush(stdout);
    unsigned int ret = send_msg(GetTime, 0);
    if (ret != 0)
	exit(-6);
    ret = receive_msg();
    //printf("get_time end\n");
    fflush(stdout);
    return u64_to_timeval_us(ret);
}

struct timeval blocking()
{
    //printf("blocking beginning\n");
    //fflush(stdout);
    unsigned int ret = send_msg(Stuck, 0);
    if (ret != 0)
        exit(-8);
    ret = receive_msg();
    //printf("blocking end\n");
    //fflush(stdout);
    return u64_to_timeval_us(ret);
}

void add_event(struct timeval t)
{
    //printf("add_event beginning\n");
    fflush(stdout);
    unsigned int ret = send_msg(AddStep, timeval_to_uint_us(t));
    if (ret != 0)
	exit(-9);
    //printf("add_event end\n");
    fflush(stdout);
}

void suppress_event(struct timeval t)
{
    //printf("suppress_event beginning\n");
    fflush(stdout);
    unsigned int ret = send_msg(DelStep, timeval_to_uint_us(t));
    if (ret != 0)
	exit(-10);
    //printf("suppress_event end\n");
    fflush(stdout);
}

int get_random()
{
    //printf("get_random beginning\n");
    fflush(stdout);
    unsigned int ret = send_msg(GetRand, 0);
    if (ret != 0)
    exit(-11);
    //printf("get_random end\n");
    fflush(stdout);
    return receive_msg();

}

void __attribute__((destructor)) send_finished() {
    //printf("Sending message: Finished\n");
    send_msg(Finished, 0);
}