#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
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
    char *ip_dst = NULL;
    char opt;
    int port_dst = 0;
    int client_id = 0;
    int sleep_time = -1;
    int nb_packets = 0;
    int answer = 0;
    while ((opt = getopt(argc, argv, "i:p:I:s:n:r")) != -1) {
        switch (opt) {
        case 'i': ip_dst = optarg; break;
        case 'p': port_dst = atoi(optarg); break;
        case 'I': client_id = atoi(optarg); break;
        case 's': sleep_time = atoi(optarg); break;
        case 'n': nb_packets = atoi(optarg); break;
        case 'r': answer = 1; break;
        default:
            print_usage(argv[0]);
            printf("args: %s", argv);
            exit(EXIT_FAILURE);
        }
    }
    printf("Client: arguments parsed\n");
    fflush(stdout);


    if (!ip_dst || !port_dst || !client_id || sleep_time < 0 || !nb_packets) {
        print_usage(argv[0]);
        printf("args: %s", argv);
        exit(EXIT_FAILURE);
    }
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

    // Set up server address
    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(port_dst);
    server_addr.sin_addr.s_addr = inet_addr(ip_dst);
    printf("Client: address configured\n");
    fflush(stdout);

    for (int i = 0 ; i < nb_packets ; i++) {
        printf("for loop\n");
        fflush(stdout);
        // Send message to the server
        int msg[2] = {client_id, i};
        sendto(sockfd, msg, sizeof(msg), 0, (struct sockaddr *)&server_addr, addr_len);
        printf("Message sent to server\n");
        fflush(stdout);

        if (answer) {
            // Receive response from server
            memset(buffer, 0, BUFFER_SIZE);
            int len = recvfrom(sockfd, buffer, BUFFER_SIZE, 0, (struct sockaddr *)&server_addr, &addr_len);
            if (len < 0) {
                perror("Receiving failed");
            } else {
                printf("Received message from server: %s\n", buffer);
                fflush(stdout);
            }
        }
        usleep(sleep_time);
    }
    printf("Client after loop\n");
    fflush(stdout);

    close(sockfd);
    printf("Client %i finished.\n", client_id);
    fflush(stdout);
    return 0;
}
