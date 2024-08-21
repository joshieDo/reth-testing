
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

**Requires ETHERSCAN_API_KEY to be set as an environment variable.**

Main goal is to have a node fill its in-memory chain and stop, so we can query it without moving forward.
