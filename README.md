# Testing utilities

### `reth-tester`
Reth node that uses ExEx & Etherscan to fill the in-memory chain and stopping.

### `cryo_test.sh`
Runs cryo commands on two nodes to check correctness and compare timings.
Modes:
1) No BLOCK_START & BLOCK_END: Uses in-memory range of the first node (BLOCK_START = STORAGE_TIP - 1, BLOCK_END = chain tip).
2) With BLOCK_START & BLOCK_END: Compares nodes using the provided range.
### examples
```bash
# requires reth-tester from above to be the first declared node
$ bash cryo_test.sh
===============================================
           BLOCK_START: 1947934
           BLOCK_END: 1947938
===============================================

TIMINGS
Type/Node  http://localhost:8545     http://localhost:8544
logs      0m0.014s                 0m0.013s ⬇️
blocks    0m0.011s                 0m0.013s ⬆️
txs       0m0.016s                 0m0.015s ⬇️

B3SUM
Type/Node  http://localhost:8545     http://localhost:8544
logs      f08c...687e              ✅
blocks    cc7d...044c              ✅
txs       7707...a685              ✅

===============================================
Script completed in 0 seconds.
Output: cryo_script_20240822_190007.log
===============================================
```


```bash
# any node 
$ bash cryo_test.sh 0 1000
===============================================
           BLOCK_START: 0
           BLOCK_END: 1000
===============================================

TIMINGS
Type/Node  http://localhost:8545     http://localhost:8544
logs      0m0.094s                 0m0.089s ⬇️
blocks    0m0.093s                 0m0.099s ⬆️
txs       0m0.158s                 0m0.154s ⬇️

B3SUM
Type/Node  http://localhost:8545     http://localhost:8544
logs      e4f3...ebdb              ✅
blocks    a9fa...b06a              ✅
txs       2ab9...b1ff              ✅

===============================================
Script completed in 1 seconds.
Output: cryo_script_20240822_190052.log
===============================================
```
