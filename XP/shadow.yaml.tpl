general:
  stop_time: 1h

network:
  graph:
    type: gml
    inline: |
      graph [
        directed 0
        node [
          id 0
          host_bandwidth_down "100 Gbit"
          host_bandwidth_up "100 Gbit"
        ]
        edge [
          source 0
          target 0
          latency "DELAY ms"
          jitter "0 ms"
        ]
      ]

hosts:
  client:
    network_node_id: 0
    ip_addr: "11.0.0.1"
    processes:
    - path: /network-time-simulator/XP/client
      args: -i 11.0.0.2 -p 4443 -o NUMBER
      start_time: 5s
      expected_final_state: {exited: 255}

  server:
    network_node_id: 0
    ip_addr: "11.0.0.2"
    processes:
    - path: /network-time-simulator/XP/server 
      args: -i 11.0.0.2 -p 4443 -o NUMBER
      start_time: 1s
