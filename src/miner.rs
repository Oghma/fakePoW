//! Miner module

use alloy_sol_types::{private::FixedBytes, SolCall};
use revm::{
    primitives::{address, keccak256, AccountInfo, Address, ExecutionResult, Output, TransactTo},
    InMemoryDB, EVM,
};
use ruint::{aliases::U256, uint};

use crate::{utils, Pow};

pub struct Miner {
    evm: EVM<InMemoryDB>,
}

impl Miner {
    pub fn new() -> Self {
        let contract_address = address!("d9145CCE52D386f254917e481eB44e9943F39138");
        // Initialise the EVM
        let mut evm = utils::initialise_evm();
        let db = evm.db.as_mut().unwrap();

        // Load contract bytecode
        let contract_bytecode = utils::read_contract();

        // Deploy the contract using a fake address
        let contract = AccountInfo::new(
            U256::ZERO,
            0,
            keccak256(&contract_bytecode.bytecode),
            contract_bytecode,
        );
        db.insert_account_info(contract_address, contract);

        evm.env.tx.caller = Address::ZERO;
        evm.env.tx.transact_to = TransactTo::Call(contract_address);

        Self { evm }
    }

    /// Search for a valid hash
    pub fn mine(&mut self, leading_zeros: usize, first_nonce: U256) -> (U256, FixedBytes<32>) {
        let max_hash = FixedBytes::<32>::from_slice(&utils::num_0s(leading_zeros));
        let increment = uint!(1_U256);

        let mut second_nonce = U256::ZERO;
        let mut output: FixedBytes<32> = FixedBytes([0; 32]);
        let mut founded = false;

        let now = std::time::Instant::now();
        // Start searching the second none
        while !founded {
            // Encode call
            let call_fun = Pow::mineCall {
                nonce1: first_nonce,
                nonce2: second_nonce,
            };
            let calldata = call_fun.abi_encode();

            self.evm.env.tx.data = calldata.into();

            // Call Pow contract to calculate hash
            let result = match self.evm.transact_ref().unwrap().result {
                ExecutionResult::Success { output, .. } => match output {
                    Output::Call(out) => out,
                    _ => panic!("EVM call failed"),
                },
                _ => panic!("EVM call failed"),
            };

            output = Pow::mineCall::abi_decode_returns(&result, true)
                .unwrap()
                .hashed;

            if output <= max_hash {
                founded = true;
            } else {
                second_nonce += increment;
            }
        }
        let elapsed = now.elapsed();
        tracing::info!("Hash generated in {:?}", elapsed);

        (second_nonce, output)
    }
}
