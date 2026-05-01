# Queue name
qname = '/some_queue'

# Random number
random_number = 123

#logs
logs = []
log_file = "/dev/null"

# Number of followers
nb_follower = 2

# Graph
graph = '1_client_1_server.gml'

# Executables
[executables]
[[executables.exe]]
name = 'client'
path = 'client'
args = ["./client", "-i", "192.168.42.2", "-p", "4443", "-o", "NUMBER"]
node_ids = [1]


[[executables.exe]]
name = 'server'
path = 'server'
args = ["./server", "-i", "192.168.42.2", "-p", "4443", "-o", "NUMBER"]
node_ids = [2]

