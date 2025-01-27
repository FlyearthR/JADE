 #include <mqueue.h>
 #include <stdio.h>
 #include <errno.h>

int main(int argc, char **argv)
{
	int ret = mq_unlink(argv[1]);
	if (!ret) {
		printf("Suppressed\n");
		return 0;
	} else if (ret == EACCES) {
		printf("The  caller  does  not  have  permission  to unlink this message queue.\n");
		return 1;
	} else if (ret == ENOENT) {
		printf("There is no message queue with the given name.\n");
		return 2;
	} else if (ret == ENAMETOOLONG) {
		printf("name was too long.\n");
		return 3;
	} else {
		printf("An unexpected error happened (probably a wrong user calling this process, with a non standart error code).\n");
		return ret;
	}
}
