graph [
  label "test"
  id 4
  node [
    id 1
    label "client"
    interface [
      id 0
      label "c-eth0"
      ip [
        type "v4"
        ip "192.168.42.1"
      ]
    ]
  ]
  node [
    id 2
    label "server"
    interface [
      id 0
      label "s-eth0"
      ip [
        type "v4"
        ip "192.168.42.2"
      ]
    ]
  ]
  edge [
    source 1
    source_if 0
    target 2
    target_if 0
    label "Link"
    metric DELAY
    type "symmetric"
  ]
]
