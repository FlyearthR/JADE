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
        ip "192.168.1.1"
      ]
      ip [
        type "v4"
        ip "192.168.1.2"
      ]
    ]
    interface [
      id 1
      label "eth-1"
      ip [
        type "v4"
        ip "10.0.0.1"
      ]
    ]
  ]
  node [
    id 2
    label "Node 2"
    interface [
      id 0
      label "wlp4s0"
      ip [
        type "v4"
        ip "172.16.0.2"
      ]
    ]
  ]
  edge [
    source 1
    source_if 0
    target 2
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 1
    source_if 1
    target 2
    target_if 0
    label "Link"
    metric 10
    type "symmetric"
  ]
]
