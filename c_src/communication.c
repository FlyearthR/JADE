#include "communication.h"

int id = 0;
int fdi = 0;
int fdo = 0;

void __attribute__((constructor)) init_fd() { // TODO: get the name of the queue from the env
    id = atoi(getenv("ID"));
    char out[10] = "";
    strcat(out, getenv("ID"));
    strcat(out, ".out");
    freopen(out, "w", stdout);
    char err[10] = "";
    strcat(err, getenv("ID"));
    strcat(err, ".err");
    freopen(err, "w", stderr);

    char FD[20] = "/some_queue_";
    strcat(FD, getenv("ID"));
    struct mq_attr attr = {
        .mq_flags = 0,
        .mq_maxmsg = 20,
        .mq_msgsize = 27,
        .mq_curmsgs = 0,
    };
    fdi = mq_open(FD, O_RDONLY|O_CREAT, (mode_t) 0600, attr);
    printf("Opened %s in read mode\n", FD);
    fflush(stdout);
    FD[12] = '0';
    FD[13] = '\0';
    fdo = mq_open(FD, O_WRONLY|O_CREAT, (mode_t) 0600, attr);
    printf("Opened %s in write mode\n", FD);
    fflush(stdout);

    pkt_list = NULL;
    next_pkt_id = 0;
}

int send_msg(Message m) // TODO: extend this
{
    printf("inside send_msg\n");
    fflush(stdout);
    Buffer b = serialize(m);
    int ret = mq_send(FDO, b.buffer, SIZE_BUFFER, (m.tag == GetTime || m.tag == GetRand) ? 2 : 1);
    printf("after mq_send\n");
    fflush(stdout);
    return ret;
}

uint64_t receive_msg()
{
    Buffer msg;
    do {
        int ret = mq_receive(FDI, msg.buffer, 27, NULL);
        if (ret == -1) {
            fprintf(stderr, "Error receiving message in receive_msg()\n");
            perror("Error: ");
            exit(-6);
        }
        Message m = deserialize(msg);
        if (m.tag == Send)
        {
            sender(m.send);
        }
        else
        {
            return m.wake_up;
        }
    } while (true);
}

struct timeval get_time() // TODO: is current time a global variable?
{
    Message m = {.tag = GetTime, .get_time = ID};
    unsigned int ret = send_msg(m);
    if (ret != 0)
	exit(-6);
    ret = receive_msg();
    return u64_to_timeval_us(ret);
}

struct timeval blocking()
{
    printf("inside blocking\n");
    Message m = {.tag = Stuck, .stuck = ID};
    unsigned int ret = send_msg(m);
    printf("ret of send_msg: %i\n", ret);
    if (ret != 0)
        exit(-8);
    ret = receive_msg();
    return u64_to_timeval_us(ret);
}

void add_event(struct timeval t)
{
    Message m = {.tag = AddStep, .add_step = {._0 = ID, ._1 = timeval_to_uint_us(t)}};
    unsigned int ret = send_msg(m);
    if (ret != 0)
	exit(-9);
}

void suppress_event(struct timeval t)
{
    Message m = {.tag = DelStep, .del_step = {._0 = ID, ._1 = timeval_to_uint_us(t)}};
    unsigned int ret = send_msg(m);
    if (ret != 0)
	exit(-10);
}

int get_random()
{
    Message m = {.tag = GetRand, .get_rand = ID};
    unsigned int ret = send_msg(m);
    if (ret != 0)
        exit(-11);
    return receive_msg();

}

void custom_send(packet pkt)
{
    LIBC_FUNCTION(ssize_t, send, int sockfd, const void* buf, size_t len, int flags);
    int ret = LIBC_FUNCTION_GET(send)(pkt.sockfd, pkt.buf, pkt.len, pkt.flags);
    free((void*) pkt.buf);
}

void custom_sendto(packet pkt)
{

    LIBC_FUNCTION(ssize_t, sendto, int sockfd, const void* buf, size_t len, int flags, const struct sockaddr *dest_addr, socklen_t addrlen);
    int ret = LIBC_FUNCTION_GET(sendto)(pkt.sockfd, pkt.buf, pkt.len, pkt.flags, pkt.dest_addr, pkt.addrlen);
    free((void*) pkt.dest_addr);
    free((void*) pkt.buf);
}

void custom_sendmsg(packet pkt)
{
    // TODO: implem
    free((void*) pkt.buf);
}

void sender(uint64_t pkt_id)
{
    packet_elem goal = {.pkt = {.id = pkt_id}};
    packet_elem* found = NULL;
    LL_SEARCH(pkt_list, found, &goal, cmp_pkt);
    if (!found)
        exit (-12);
    switch(found->pkt.tos) {
        case send_t:
            custom_send(found->pkt);
            break;

        case sendto_t:
            custom_sendto(found->pkt);
            break;

        case sendmsg_t:
            custom_sendmsg(found->pkt);
            break;
    }
    LL_DELETE(pkt_list, found);
    free(found);
    Message m = {.tag = Sent, .sent = {._0 = ID, ._1 = pkt_id}};
    send_msg(m);
}

void __attribute__((destructor)) send_finished() {
    int count;
    packet_elem* p;
    Buffer msg;
    do {
        int ret = mq_receive(FDI, msg.buffer, 10, NULL);
        if (ret == -1) {
            fprintf(stderr, "Error receiving message in send_finished\n");
            perror("Error: ");
            exit(-6);
        }
        Message m = deserialize(msg);
        if (m.tag == Send)
        {
            sender(m.send);
        }
        else
        {
            // ignore
        }
        LL_COUNT(pkt_list, p, count);
    } while(count);
    Message m = {.tag = Finished, .finished = ID};
    send_msg(m);
}