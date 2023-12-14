//! Application main

use alloy_sol_types::sol;
use rand::Rng;

mod miner;
mod utils;

// Generate contract abi
sol!("contracts/Pow.sol");

fn main() {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let leading_zeros = std::env::var("NUM_0S").unwrap().parse().unwrap();
    tracing::info!("Find hash with {} leading zeros", leading_zeros);

    let miner = miner::Miner::new();

    // Generate first nonce
    let mut rng = rand::thread_rng();
    let first_nonce = rng.gen();

    tracing::info!("First nonce generated: {}", first_nonce);

    let Some((second_nonce, hash)) = miner.mine(leading_zeros, first_nonce) else {
        panic!("Failed to find an hash")
    };

    tracing::info!("Second nonce founded: {}", second_nonce);
    tracing::info!("Hash generated: {:?}", hash);
}
