#! /bin/bash

MAX_DELAY=$1 #paper value: 40
MAX_PACKETS=$2 #paper value: 20000

TIMEFORMAT="%2R"

#tc emulation process - star topology
#-------------------------------------
call_tc_star() {
	pkts=$1
	delay=$2
	num_clients=$3

	# Default to 1 client if not specified
	num_clients=${num_clients:-1}

	# Create server namespace
	ip netns add server
	ip netns exec server ip link set lo up

	# Create client namespaces and veth pairs
	for i in $(seq 1 $num_clients); do
		ip netns add client$i

		# Create veth pair connecting client to server
		ip link add c${i}-eth0 type veth peer name s${i}-eth0

		# Move veth ends to respective namespaces
		ip link set c${i}-eth0 netns client$i
		ip link set s${i}-eth0 netns server

		# Configure IP addresses (each client gets 192.168.(40+i).0/24)
		ip netns exec client$i ip addr add 192.168.$((40+i)).1/24 dev c${i}-eth0
		ip netns exec server ip addr add 192.168.$((40+i)).2/24 dev s${i}-eth0

		# Bring up interfaces
		ip netns exec client$i ip link set lo up
		ip netns exec client$i ip link set c${i}-eth0 up
		ip netns exec server ip link set s${i}-eth0 up

		# Add network delay
		ip netns exec client$i tc qdisc add dev c${i}-eth0 root netem delay ${delay}ms
		ip netns exec server tc qdisc add dev s${i}-eth0 root netem delay ${delay}ms
	done

	# Start server
	ip netns exec server ./server_multi -i 0.0.0.0 -p 4443 -o $pkts -c $num_clients &
	pid=$!
	sleep 0

	# Start all clients
	for i in $(seq 1 $num_clients); do
		ip netns exec client$i ./client -i 192.168.$((40+i)).2 -p 4443 -o $pkts -I $i > /dev/null 2> /dev/null &
	done

	# Wait for all clients to finish
	wait

	# Cleanup: remove tc qdiscs
	for i in $(seq 1 $num_clients); do
		ip netns exec client$i tc qdisc del dev c${i}-eth0 root
		ip netns exec server tc qdisc del dev s${i}-eth0 root
	done

	# Remove namespaces
	for i in $(seq 1 $num_clients); do
		ip netns del client$i
	done
	ip netns del server
}

#Setting up
#----------
echo "Compiling toy examples"
gcc miniP_server_multi.c -o server_multi
gcc miniP0_client.c -o client

#Multiple clients - tc
#---------------------
pkts=10
delay=1
num_clients=3
call_tc_star $pkts $delay $num_clients > tc_star1.log
call_tc_star $pkts $delay $num_clients > tc_star2.log
diff tc_star1.log tc_star2.log > /dev/null
if [ $? -ne 0 ]
then
	echo "Emulation gives different ordering of packet arrival"
fi


#Multiple clients - JADE
#-----------------------
cp ../target/release/simulator .
cp ../target/syscalls/syscalls.so .
./simulator 3_clients_1_server.toml