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
    ]
  ]
  node [
    id 11
    label "Server"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.20"
      ]
    ]
  ]
  edge [
    source 1
    source_if 0
    target 11
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
]
