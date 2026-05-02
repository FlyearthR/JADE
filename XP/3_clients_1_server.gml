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
    id 2
    label "Node 1"
    interface [
      id 0
      label "eth-0"
      ip [
        type "v4"
        ip "10.0.0.2"
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
    ]
  ]
  edge [
    source 1
    source_if 0
    target 4
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 2
    source_if 0
    target 4
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
  edge [
    source 3
    source_if 0
    target 4
    target_if 0
    label "Link"
    metric 20
    type "symmetric"
  ]
]
