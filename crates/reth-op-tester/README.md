
```bash
$ reth-op-tester node --help
(...)
Testing ExEx:
      --etherscan-url <ETHERSCAN_API_URL>

      --num-blocks <NUM_BLOCKS>
          Uses etherscan to sync up to `num_blocks`. **Should not** be used with a CL

          [default: 3]

      --against-rpc <AGAINST_RPC>
          Runs equality tests across many RPCs calls after syncing `num_blocks`
(...)     
```

Uses ExEx & etherscan to move the chain forward **until** it has collected a specific number of blocks. It defaults to the default persistence threshold if no argument is passed.

**Requires ETHERSCAN_API_KEY to be set as an environment variable.**

Main goal is to have a node fill its in-memory chain and stop, so we can query it without moving forward.
