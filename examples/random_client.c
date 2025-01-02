#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <getopt.h>
#include <arpa/inet.h>

#define BUFFER_SIZE 1024

void print_usage(char exe[]) {
    fprintf(stderr, "Usage: %s -i destination_IP -p destination_port -I node_ID -s sleep_time (in usec) -n number_of_packets [-r]\n", exe);
}

void print_args(int argc, char* argv[]) {
    for (int i = 0 ; i < argc ; i++) {
        printf("%s ", argv[i]);
    }
    printf("\n");
    fflush(stdout);
}

int main(int argc, char* argv[]) {
    print_args(argc, argv);
    printf("Client: launched\n");
    fflush(stdout);


    printf("Client: arguments ok\n");
    fflush(stdout);

    int sockfd;
    struct sockaddr_in server_addr;
    char buffer[BUFFER_SIZE];
    socklen_t addr_len = sizeof(server_addr);

    // Create a UDP socket
    if ((sockfd = socket(AF_INET, SOCK_DGRAM, 0)) < 0) {
        perror("Socket creation failed");
        exit(EXIT_FAILURE);
    }
    printf("Client: socket opened\n");
    fflush(stdout);


    srand(14);
    printf("random number : %i\n", rand());
    printf("random number : %i\n", rand());
    printf("random number : %i\n", rand());

    srand(32);
    printf("random number : %i\n", rand());
    printf("random number : %i\n", rand());
    printf("random number : %i\n", rand());

    srand(14);
    printf("random number : %i\n", rand());
    printf("random number : %i\n", rand());
    printf("random number : %i\n", rand());

    close(sockfd);
    // printf("Client %i finished.\n", client_id);
    fflush(stdout);
    return 0;
}