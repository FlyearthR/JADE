 #include <mqueue.h>

int main(int argc, char **argv)
{
	mq_unlink(argv[1]);
	return 0;
}
