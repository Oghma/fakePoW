# pNetwork Fake PoW

The repository contains the code for an assessment that I made for the pNetowk
interview.

The task was:

``` text

Interview Project: Build a “miner” using Rust & Solidity

Solidity:

Write a smart contract with a `mine` function, which accepts two nonces as
parameters, hashes them together, and returns that hash.

Rust:

Write a program which calls the above `mine` function in the smart contract. The
program should create a random number for the first nonce, and start with `0`
for the second nonce. The program should call the solidity function and check
the returned hash for how many `0` characters it begins with. The program should
have a configurable that defines how many zeros that hash should begin with. The
program should run, incrementing the second nonce, until it finds one which when
hashed with the randomly generated first nonce returns a hash with >= the number
of required zeroes at the start of the hash. The program should return with the
random number it generated, and the nonce which results in a hash with the
required number of zeroes, and the hash itself.

Goals:

Make your program as fast as possible.


```

## Usage

To select the leading 0s open `.env` and change `NUM_0S` with the desired
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
