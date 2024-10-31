#include "communication.h"

int id = 0;
#define ID id

#define FDI 4
#define FDO 3

void __attribute__((constructor)) init_fd() {
    id = atoi(getenv("ID"));
    char out[10] = "";
    strcat(out, getenv("ID"));
    strcat(out, ".out");
    freopen(out, "w", stdout);
    char err[10] = "";
    strcat(err, getenv("ID"));
    strcat(err, ".err");
    freopen(err, "w", stderr);

    pkt_list = NULL;
    next_pkt_id = 0;
}

int send_msg(Message_Tag m, unsigned int attr)
{
    Message msg = {.tag=m, .get_time=ID};
    if (m == AddStep || m == DelStep) {
        msg.add_step._1 = attr;
    }
    Buffer b = serialize(msg);
    int ret = mq_send(FDO, b.buffer, 10, (m == GetTime || m == GetRand) ? 2 : 1);
    return ret;
}

uint64_t receive_msg()
{
    Buffer msg;
    do {
        int ret = mq_receive(FDI, msg.buffer, 10, NULL);
        if (ret == -1) {
            fprintf(stderr, "Error receiving message\n");
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

struct timeval get_time()
{
    unsigned int ret = send_msg(GetTime, 0);
    if (ret != 0)
	exit(-6);
    ret = receive_msg();
    return u64_to_timeval_us(ret);
}

struct timeval blocking()
{
    unsigned int ret = send_msg(Stuck, 0);
    if (ret != 0)
        exit(-8);
    ret = receive_msg();
    return u64_to_timeval_us(ret);
}

void add_event(struct timeval t)
{
    unsigned int ret = send_msg(AddStep, timeval_to_uint_us(t));
    if (ret != 0)
	exit(-9);
}

void suppress_event(struct timeval t)
{
    unsigned int ret = send_msg(DelStep, timeval_to_uint_us(t));
    if (ret != 0)
	exit(-10);
}

int get_random()
{
    unsigned int ret = send_msg(GetRand, 0);
    if (ret != 0)
        exit(-11);
    return receive_msg();

}

void sender(uint64_t pkt_id)
{
    packet_elem goal = {.pkt = {.id = pkt_id}};
    packet_elem* found = NULL;
    LL_SEARCH(pkt_list, found, &goal, cmp_pkt);
    if (!found)
        exit (-12);
    
    LIBC_FUNCTION(ssize_t, send, int sockfd, const void buf, size_t len, int flags);
    int ret = LIBC_FUNCTION_GET(send)(found->pkt.sockfd, found->pkt.buf, found->pkt.len, found->pkt.flags);
    LL_DELETE(pkt_list, found);
    free(found->buf);
    free(found);

    send_msg(Sent, pkt_id);
}
