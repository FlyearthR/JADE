#! /bin/bash

MAX_DELAY=$1 #paper value: 40
MAX_PACKETS=$2 #paper value: 20000

TIMEFORMAT="%2R"


#tc emulation process - star topology
#-------------------------------------
call_tc() {
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
	ip netns exec server ./server -i 192.168.41.2 -p 4443 -o $pkts > /dev/null 2> /dev/null &
	pid=$!
	sleep 0

	# Start all clients
	for i in $(seq 1 $num_clients); do
		ip netns exec client$i ./client -i 192.168.$((40+i)).2 -p 4443 -o $pkts > /dev/null 2> /dev/null &
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
gcc miniP0_server.c -o server 2> /dev/null
gcc miniP0_client.c -o client 2> /dev/null
cp ../target/release/simulator .
cp ../target/syscalls/syscalls.so .


#Figure 3 - tc
#-------------
echo "Evaluating tc curve of Figure 3"
pkts=500
num_clients=1  # Number of clients in star topology
echo -n "" > tc_${pkts}.csv

for i in $(seq 1 $MAX_DELAY);
do
	echo ${i}ms
		echo -n "$i, " >> tc_${pkts}.csv
	{ time call_tc $pkts $i $num_clients ; } 2>> tc_${pkts}.csv
done


#Figure 4 - tc
#-------------
echo "Evaluating tc curve of Figure 4"
delay=1
num_clients=1  # Number of clients in star topology
echo -n "" > tc_nb${delay}.csv

for i in $(seq 500 500 $MAX_PACKETS);
do
    	echo ${i} pkts
		echo -n "$i, " >> tc_nb${delay}.csv
	{ time call_tc $i $delay $num_clients ; } 2>> tc_nb${delay}.csv
done


#Figure 3 - JADE
#---------------
echo "Evaluating jade curve of Figure 3"
sed "s/NUMBER/500/g" 1_client_1_server.toml.tpl > 1_client_1_server.toml
pkts=500
echo -n "" > jade_${pkts}.csv

for i in $(seq 1 $MAX_DELAY);
do
    echo ${i}ms
    sed "s/DELAY/$i/g" 1_client_1_server.gml.tpl > 1_client_1_server.gml
    echo -n "$i, " >> jade_${pkts}.csv
    /usr/bin/time -f %e -o tps.tmp ./simulator 1_client_1_server.toml > /dev/null
    cat tps.tmp >> jade_${pkts}.csv
done


#Figure 4 - JADE
#---------------
echo "Evaluating jade curve of Figure 4"
sed "s/DELAY/1/g" 1_client_1_server.gml.tpl > 1_client_1_server.gml
delay=1
echo -n "" > jade_nb_5.csv

for i in $(seq 500 500 $MAX_PACKETS);
do
    echo ${i} pkts
    sed "s/NUMBER/$i/g" 1_client_1_server.toml.tpl > 1_client_1_server.toml
    echo -n "$i, " >> jade_nb_5.csv
    for j in $(seq 1 4);
    do
    	/usr/bin/time -f %e -o tps.tmp ./simulator 1_client_1_server.toml > /dev/null
	T=$(cat tps.tmp)
	echo -n $T, >> jade_nb_5.csv
    done
    /usr/bin/time -f %e -o tps.tmp ./simulator 1_client_1_server.toml > /dev/null
    cat tps.tmp >> jade_nb_5.csv
done


#Figure 3 - shadow
#-----------------
echo "Evaluating shadow curve of Figure 3"
sed "s/NUMBER/500/g" shadow.yaml.tpl > shadow.yaml.tpl2
pkts=500
echo -n "" > shadow_${pkts}.csv

for i in $(seq 1 $MAX_DELAY);
do
    echo ${i}ms
    sed "s/DELAY/$i/g" shadow.yaml.tpl2 > shadow.yaml
    echo -n "$i, " >> shadow_${pkts}.csv
    /usr/bin/time -f %e -o tps.tmp /root/.local/bin/shadow shadow.yaml > /dev/null
    rm -rf shadow.data
    cat tps.tmp >> shadow_${pkts}.csv
done


#Figure 4 - shadow
#-----------------
echo "Evaluating shadow curve of Figure 4"
sed "s/DELAY/1/g" shadow.yaml.tpl > shadow.yaml.tpl2
delay=1
echo -n "" > shadow_nb_5.csv

for i in $(seq 500 500 $MAX_PACKETS);
do
    echo ${i} pkts
    sed "s/NUMBER/$i/g" shadow.yaml.tpl2 > shadow.yaml
    echo -n "$i, " >> shadow_nb_5.csv
    for j in $(seq 1 4);
    do
    	/usr/bin/time -f %e -o tps.tmp /root/.local/bin/shadow shadow.yaml > /dev/null
	rm -rf shadow.data
	T=$(cat tps.tmp)
	echo -n $T, >> shadow_nb_5.csv
    done
    /usr/bin/time -f %e -o tps.tmp /root/.local/bin/shadow shadow.yaml > /dev/null
    rm -rf shadow.data
    cat tps.tmp >> shadow_nb_5.csv
done