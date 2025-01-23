#include "communication.h"

// TODO: fix memory leaks due to (de)serialization

int id = 0;
int seed = 0;
int fdi = 0;
int fdo = 0;
int current_time;

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
    //printf("Opened %s in read mode\n", FD);
    //fflush(stdout);
    FD[12] = '0';
    FD[13] = '\0';
    fdo = mq_open(FD, O_WRONLY|O_CREAT, (mode_t) 0600, attr);
    //printf("Opened %s in write mode\n", FD);
    //fflush(stdout);

    pkt_list = NULL;
    fd_ip_list = NULL;
    next_pkt_id = 0;

    current_time = receive_msg();
}

void print_msg(Message m)
{
    /*switch (m.tag) {
        case Stuck:
            printf("Stuck(%i)\n", m.stuck);
            break;
        case AddStep:
            printf("AddStep(%i, %i)\n", m.add_step._0, m.add_step._1);
            break;
        case DelStep:
            printf("DelStep(%i)\n", m.del_step._0, m.del_step._1);
            break;
        case HasToSend4:
            printf("HasToSend4(%i, %i, %i.%i.%i.%i, %i)\n", m.has_to_send4._0, m.has_to_send4._1,
                m.has_to_send4._2.segments[0], m.has_to_send4._2.segments[1], m.has_to_send4._2.segments[2],
                m.has_to_send4._2.segments[3], m.has_to_send4._3);
            break;
        case HasToSend6:
            printf("HasToSend6(%i, %i, %i:%i:%i:%i:%i:%i:%i:%i, %i)\n", m.has_to_send6._0, m.has_to_send6._1,
                m.has_to_send6._2.segments[0], m.has_to_send6._2.segments[1], m.has_to_send6._2.segments[2],
                m.has_to_send6._2.segments[3], m.has_to_send6._2.segments[4], m.has_to_send6._2.segments[5],
                m.has_to_send6._2.segments[6], m.has_to_send6._2.segments[7], m.has_to_send6._3);
            break;
        case Send:
            printf("Send(%i)\n", m.send);
            break;
        case Sent:
            printf("Sent(%i, %i)\n", m.sent._0, m.sent._1);
            break;
        case GetTime:
            printf("GetTime(%i)\n", m.get_time);
            break;
        case GetRand:
            printf("GetRand(%i)\n", m.get_rand);
            break;
        case WakeUp:
            printf("WakeUp(%i)\n", m.wake_up);
            break;
        case Finished:
            printf("Finished(%i)\n", m.finished);
            break;
    }*/
}

int send_msg(Message m) // TODO: extend this
{
    //printf("inside send_msg\n");
    print_msg(m);
    fflush(stdout);
    Buffer* b = serialize(m);
    /*for (int i = 0 ; i < 27 ; i++) {
        uint8_t tmp = b->buffer[i];
        printf("%i ", tmp);
    }
    printf("\nmessage printed\n", b);*/
    int ret = mq_send(FDO, b->buffer, SIZE_BUFFER, (m.tag == GetTime || m.tag == GetRand) ? 2 : 1);
    //printf("after mq_send\n");
    //fflush(stdout);
    return ret;
}

uint64_t receive_msg()
{
    Buffer msg;
    do {
        int ret = mq_receive(FDI, msg.buffer, SIZE_BUFFER, NULL);
        if (ret == -1) {
            //fprintf(stderr, "Error receiving message in receive_msg()\n");
            perror("Error: ");
            exit(-6);
        }
        /*printf("received a message\n");
        for (int i = 0 ; i < SIZE_BUFFER ; i++)
        {
            printf("%3i ", msg.buffer[i]);
        }
        printf("\n");*/
        Message* m = deserialize(msg);
        print_msg(*m);
        if (m->tag == Send)
        {
            sender(m->send);
        }
        else
        {
            current_time = m->wake_up;
            return m->wake_up;
        }
    } while (true);
}

struct timeval get_time()
{
    /*Message m = {.tag = GetTime, .get_time = ID};
    printf("get_time called\n");
    unsigned int ret = send_msg(m);
    if (ret != 0)
	exit(-6);
    ret = receive_msg();*/
    return u64_to_timeval_us(current_time);
}

uint64_t get_u64_time()
{
    return current_time;
}

struct timeval blocking()
{
    //printf("inside blocking\n");
    Message m = {.tag = Stuck, .stuck = ID};
    //printf("blocking called \n");
    unsigned int ret = send_msg(m);
    //printf("ret of send_msg: %i\n", ret);
    if (ret != 0)
        exit(-8);
    ret = receive_msg();

    return u64_to_timeval_us(ret);
}

void add_event(struct timeval t)
{
    Message m = {.tag = AddStep, .add_step = {._0 = ID, ._1 = timeval_to_uint_us(t)}};
    //printf("add_event called \n");
    unsigned int ret = send_msg(m);
    if (ret != 0)
	exit(-9);
}

void suppress_event(struct timeval t)
{
    Message m = {.tag = DelStep, .del_step = {._0 = ID, ._1 = timeval_to_uint_us(t)}};
    //printf("suppress_event called \n");
    unsigned int ret = send_msg(m);
    if (ret != 0)
	exit(-10);
}

int get_random()
{
    fflush(stdout);
    Message m = {.tag = GetRand, .get_rand = {._0= ID, ._1 = seed}};
    //printf("get_random called \n");
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
    printf("custom_sendmsg\n");
    LIBC_FUNCTION(ssize_t, sendmsg, int sockfd, const struct msghdr *msg, int flags);
    int ret = LIBC_FUNCTION_GET(sendmsg)(pkt.sockfd,(struct msghdr *)pkt.buf,pkt.flags);
    if (ret)
        perror("sendmsg: ");
    printf("return value custom_sendmsg : %d\n", ret);
    //TODO : free better
    free((void*) pkt.buf);
}

void sender(uint64_t pkt_id)
{
    printf("sender called\n");
    packet_elem goal = {.pkt = {.id = pkt_id}};
    packet_elem* found = NULL;
    LL_SEARCH(pkt_list, found, &goal, cmp_pkt);
    if (!found)
        exit (-12);
    printf("pkt tos : %d\n",found->pkt.tos);
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
    //printf("sender called \n");
    send_msg(m);
}

void __attribute__((destructor)) send_finished() {
    int count;
    packet_elem* p;
    Buffer msg;
    LL_COUNT(pkt_list, p, count);
    while(count) {
        Message m_stuck = {.tag = Stuck, .stuck = ID};
        send_msg(m_stuck);
        int ret = mq_receive(FDI, msg.buffer, 10, NULL);
        if (ret == -1) {
            fprintf(stderr, "Error receiving message in send_finished\n");
            perror("Error: ");
            exit(-6);
        }
        Message* m = deserialize(msg);
        if (m->tag == Send)
        {
            sender(m->send);
        }
        else
        {
            // ignore
        }
    }
    Message m = {.tag = Finished, .finished = ID};
    //printf("destructor called \n");
    send_msg(m);
}