
```bash
$ reth node --help
(...)
Testing ExEx:
      --num-blocks <NUM_BLOCKS>
          [default: 3]

      --etherscan-url <ETHERSCAN_API_URL>
(...)     
```

Uses ExEx & etherscan to move the chain forward **until** it has collected a specific number of blocks. It defaults to the default persistence threshold if no argument is passed.

Main goal is to be able to have a node fill its in-memory chain, and be able to query it without it moving forward.
