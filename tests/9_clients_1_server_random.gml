graph [
  label "test"
  id 4
  node [
    id 1
    label "Node 1"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.1"
      ]
      ip [
        type "v6"
        ip "2001:0db8:0000:85a3:0000:0000:ac1f:8001"
      ]
    ]
  ]
  node [
    id 2
    label "Node 1"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.2"
      ]
      ip [
        type "v6"
        ip "2001:0db8:0000:85a3:0000:0000:ac1f:8002"
      ]
    ]
  ]
  node [
    id 3
    label "Node 1"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.3"
      ]
      ip [
        type "v6"
        ip "2001:0db8:0000:85a3:0000:0000:ac1f:8003"
      ]
    ]
  ]
  node [
    id 4
    label "Node 1"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.4"
      ]
      ip [
        type "v6"
        ip "2001:0db8:0000:85a3:0000:0000:ac1f:8004"
      ]
    ]
  ]
  node [
    id 5
    label "Node 1"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.5"
      ]
      ip [
        type "v6"
        ip "2001:0db8:0000:85a3:0000:0000:ac1f:8005"
      ]
    ]
  ]
  node [
    id 6
    label "Node 1"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.6"
      ]
      ip [
        type "v6"
        ip "2001:0db8:0000:85a3:0000:0000:ac1f:8006"
      ]
    ]
  ]
  node [
    id 7
    label "Node 1"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.7"
      ]
      ip [
        type "v6"
        ip "2001:0db8:0000:85a3:0000:0000:ac1f:8007"
      ]
    ]
  ]
  node [
    id 8
    label "Node 1"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.8"
      ]
      ip [
        type "v6"
        ip "2001:0db8:0000:85a3:0000:0000:ac1f:8008"
      ]
    ]
  ]
  node [
    id 9
    label "Node 1"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.9"
      ]
      ip [
        type "v6"
        ip "2001:0db8:0000:85a3:0000:0000:ac1f:8009"
      ]
    ]
  ]
  node [
    id 10
    label "Server"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.20"
      ]
      ip [
        type "v6"
        ip "2001:0db8:0000:85a3:0000:0000:ac1f:800a"
      ]
    ]
  ]
  edge [
    source 1
    source_if 0
    target 10
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 2
    source_if 0
    target 10
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 3
    source_if 0
    target 10
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 4
    source_if 0
    target 10
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 5
    source_if 0
    target 10
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 6
    source_if 0
    target 10
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 7
    source_if 0
    target 10
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 8
    source_if 0
    target 10
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 9
    source_if 0
    target 10
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
]
