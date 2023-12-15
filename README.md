# pNework Fake PoW

## Usage

To decide the leading 0s open `.env` and change `NUM_0S` with the desired
number.

After that run `cargo run --release` and wait for the hash

## How it works

- Pow smart contract is located in `/contracts`. The contract contains the
  function `mine` that calculated the hash
- `Miner` spawns a number of `Worker`s that call the contract
- Instead of using a local node, `revm` is used. We want speed and revm is speed
  (not as much as evmone). If you ride like lightning, you're going to crash
  like thunder.[cit]

## Benchmark

See `benchmarks.org` to read some benchmarks during development. Last version
calculate an hash with 6 leading zeros in 1.03s on my machine. First nonce used
is 74767092597723011460313840600695628802137084954000954789539308737483476589197
