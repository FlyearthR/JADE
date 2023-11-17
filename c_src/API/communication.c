#include "communication.h"

int id = 0;
#define ID id

#define FDI 4
#define FDO 3

void __attribute__((constructor)) init_fd() {
    printf("before loading and freopen\n");
    id = atoi(getenv("ID"));
    char out[10] = "";
    strcat(out, getenv("ID"));
    strcat(out, ".out");
    freopen(out, "w", stdout);
    char err[10] = "";
    strcat(err, getenv("ID"));
    strcat(err, ".err");
    freopen(err, "w", stderr);
    printf("before loading\n");
    fflush(stdout);
}

int send_msg(Message_Tag m, unsigned int attr)
{
    printf("Send message %i\n", m);
    fflush(stdout);
    fflush(stderr);
    Message msg = {.tag=m, .get_time=ID};
    printf("ID = %i\n", ID);
    fflush(stdout);
    fflush(stderr);
    Buffer b = serialize(msg);
    for(int i = 0 ; i < 10 ; i++) {
        printf("%i ", b.buffer[i]);
    }
    printf("\n");
    fflush(stdout);
    fflush(stderr);
    int ret = mq_send(FDO, b.buffer, 10, (m == GetTime || m == GetRand) ? 2 : 1);
    return ret;
}

uint64_t receive_msg()
{
    Buffer msg;
    int ret = mq_receive(FDI, msg.buffer, 10, NULL);
	if (ret == -1) {
		fprintf(stderr, "Error receiving message\n");
		perror("Error: ");
		exit(-6);
	}
    return deserialize(msg).wake_up;
}

struct timeval get_time()
{
    printf("Sending message: GetTime\n");
    fflush(stdout);
    fflush(stderr);
    unsigned int ret = send_msg(GetTime, 0);
    if (ret != 0)
	exit(-6);
    ret = receive_msg();
    printf("time received u64: %i\n", ret);
    return uint_to_timeval_us(ret);
}

struct timeval waiting()
{
    printf("Sending message: Progressed\n");
    unsigned int ret = send_msg(Progressed, 0);
    if (ret != 0)
	exit(-7);
    ret = receive_msg();
    return uint_to_timeval_us(ret);
}

struct timeval blocking()
{
    printf("Sending message: Stuck\n");
    unsigned int ret = send_msg(Stuck, 0);
    if (ret != 0)
        exit(-8);
    ret = receive_msg();
    return uint_to_timeval_us(ret);
}

void add_event(struct timeval t)
{
    printf("Sending message: AddStep\n");
    printf("time: {%i,; %i}\n", t.tv_sec, t.tv_usec);
    printf("u64 t: %i\n", timeval_to_uint_us(t));
    fflush(stdout);
    fflush(stderr);
    unsigned int ret = send_msg(AddStep, timeval_to_uint_us(t));
    if (ret != 0)
	exit(-9);
}

void suppress_event(struct timeval t)
{
    printf("Sending message: DelStep\n");
    unsigned int ret = send_msg(DelStep, timeval_to_uint_us(t));
    if (ret != 0)
	exit(-10);
}

int get_random()
{
    printf("Sending message: GetRand\n");
    unsigned int ret = send_msg(GetRand, 0);
    if (ret != 0)
    exit(-11);
    return receive_msg();

}

void __attribute__((destructor)) send_finished() {
    printf("Sending message: Finished\n");
    send_msg(Finished, 0);
}