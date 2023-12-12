//! Application main

use alloy_sol_types::private::FixedBytes;
use alloy_sol_types::{sol, SolCall};
use rand::Rng;
use revm::{
    primitives::{
        address, keccak256, AccountInfo, Address, Bytes, ExecutionResult, Output, TransactTo, U256,
    },
    InMemoryDB, EVM,
};

mod utils;

// Generate contract abi
sol!("contracts/Pow.sol");

/// Initialise the EVM and deploy `PoW` smart contract
fn initialise_and_deploy() -> (EVM<InMemoryDB>, Address) {
    let pow_addr = address!("d9145CCE52D386f254917e481eB44e9943F39138");
    // Initialise the evm
    let mut evm = utils::initialise_evm();
    let db = evm.db.as_mut().unwrap();

    // Load contract bytecode
    let bytecode = utils::read_contract();

    // Deploy contract using a fake address
    let pow = AccountInfo::new(U256::ZERO, 0, keccak256(&bytecode.bytecode), bytecode);
    db.insert_account_info(pow_addr, pow);

    (evm, pow_addr)
}

fn main() {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let num_0s = std::env::var("NUM_0S").unwrap().parse().unwrap();
    tracing::info!("Find hash with {} leading zeros", num_0s);

    let (mut evm, pow_addr) = initialise_and_deploy();

    // Generate first nonce
    let mut output: FixedBytes<32> = FixedBytes([0; 32]);
    let mut rng = rand::thread_rng();

    let nonce1 = rng.gen();
    let mut second_nonce = U256::ZERO;

    tracing::info!("First nonce generated: {}", nonce1);

    evm.env.tx.caller = Address::ZERO;
    evm.env.tx.transact_to = TransactTo::Call(pow_addr);

    let max: FixedBytes<32> = FixedBytes::from_slice(&utils::num_0s(num_0s));

    let mut nonce2 = U256::ZERO;
    let mut founded = false;
    let increment = U256::from(1);

    let now = std::time::Instant::now();
    // Start searching the second nonce
    while !founded {
        //for nonce2 in 0..u32::MAX {
        let call_fun = Pow::mineCall { nonce1, nonce2 };
        let calldata = call_fun.abi_encode();

        evm.env.tx.data = calldata.into();

        let output_raw: Result<Bytes, revm::primitives::EVMError<std::convert::Infallible>> =
            evm.transact_ref().map(|result| match result.result {
                ExecutionResult::Success { output, .. } => match output {
                    Output::Call(o) => o,
                    _ => panic!("EVM call failed"),
                },
                _ => panic!("EVM call failed"),
            });

        output = Pow::mineCall::abi_decode_returns(&output_raw.unwrap(), true)
            .unwrap()
            .hashed;

        if output <= max {
            second_nonce = nonce2;
            founded = true;
        } else {
            nonce2 = nonce2 + increment;
        }
    }
    let elapsed = now.elapsed();

    tracing::info!("Second nonce founded: {}", second_nonce);
    tracing::info!("Hash generated: {}", output);
    tracing::info!("Elpased time {:?}", elapsed);
}
