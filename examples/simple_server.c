#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <sys/select.h>

void print_usage(char exe[]) {
    fprintf(stderr, "Usage: %s -p server_port -m nb_of_message_per_client -n nb_of_clients -l log_file [-r]\n", exe);
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
    char opt;
    int server_port = 0;
    int answer = 0;
    int nb_client = 0;
    int nb_msg = 0;
    char* logs = NULL;
    while ((opt = getopt(argc, argv, "p:m:n:l:r")) != -1) {
        switch (opt) {
        case 'p': server_port = atoi(optarg); break;
        case 'm': nb_msg = atoi(optarg); break;
        case 'n': nb_client = atoi(optarg); break;
        case 'l': logs = optarg; break;
        case 'r': answer = 1; break;
        default:
            print_usage(argv[0]);
            printf("args: %s", argv);
            exit(EXIT_FAILURE);
        }
    }

    printf("server_port: %i, nb_msg: %i, nb_client: %i, logs: %s, answer: %i\n", server_port, nb_msg, nb_client, logs, answer);
    fflush(stdout);
fflush(stderr);

    if (!server_port || !nb_client || !nb_msg || !logs) {
        print_usage(argv[0]);
        printf("args: %s", argv);
        exit(EXIT_FAILURE);
    }

    FILE *log_file = fopen(logs, "a");
    if (log_file == NULL) {
        perror("Failed to open log file");
        exit(EXIT_FAILURE);
    }

    int sockfd;
    struct sockaddr_in server_addr, client_addr;
    fd_set readfds;
    int buffer[2];
    socklen_t addr_len = sizeof(client_addr);

    // Create a UDP socket
    if ((sockfd = socket(AF_INET, SOCK_DGRAM, 0)) < 0) {
        perror("Socket creation failed");
        exit(EXIT_FAILURE);
    }

    // Set up server address
    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_addr.s_addr = INADDR_ANY;
    server_addr.sin_port = htons(server_port);

    // Bind the socket to the port
    if (bind(sockfd, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        perror("Bind failed");
        close(sockfd);
        exit(EXIT_FAILURE);
    }

    printf("Server is listening on port %d\n", server_port);
    fflush(stdout);
fflush(stderr);

    for (int i = 0 ; i < nb_msg*nb_client ; i++) {
        printf("for loop\n");
        fflush(stdout);
            int len = recvfrom(sockfd, buffer, sizeof(buffer), 0, (struct sockaddr *)&client_addr, &addr_len);
            printf("after recvfrom\n");
        fflush(stdout);
            if (len < 0) {
                perror("Receiving failed");
                close(sockfd);
                exit(EXIT_FAILURE);
            }
            printf("Received message from client %i: %i\n", buffer[0], buffer[1]);
    fflush(stdout);
fflush(stderr);

            fprintf(log_file, "%i %i\n", buffer[0], buffer[1]);
            fflush(log_file); 

            if (answer) {
                // Send a response back to the client
                const char *response = "Message received";
                sendto(sockfd, response, strlen(response), 0, (struct sockaddr *)&client_addr, addr_len);
            }
    }
    printf("after for loop\n");
    fflush(stdout);
fflush(stderr);

    close(sockfd);
    printf("Server finished.\n");
    fflush(stdout);
fflush(stderr);
    return 0;
}
