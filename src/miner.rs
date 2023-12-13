//! Miner module

use alloy_sol_types::SolCall;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use revm::{
    primitives::{
        address, keccak256, AccountInfo, Address, ExecutionResult, FixedBytes, Output, TransactTo,
    },
    InMemoryDB, EVM,
};
use ruint::aliases::U256;

use crate::{utils, Pow::mineCall};

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
    pub fn mine(
        &mut self,
        leading_zeros: usize,
        first_nonce: U256,
    ) -> Option<(U256, FixedBytes<32>)> {
        let max_hash = utils::num_0s(leading_zeros);
        let take_first = (leading_zeros + (leading_zeros & 1)) / 2;
        let range = utils::UintRange::new(U256::ZERO, U256::MAX);

        // First 36 bytes are fixed, prefetch it
        let mut calldata: [u8; 68] = [0; 68];
        calldata[..4].copy_from_slice(&mineCall::SELECTOR);
        calldata[4..36].copy_from_slice(&first_nonce.to_be_bytes::<32>());

        let now = std::time::Instant::now();
        // Start finding in parallel
        let result = range.par_bridge().find_map_any(|second_nonce| {
            let mut calldata = calldata.clone();
            calldata[36..68].copy_from_slice(&second_nonce.to_be_bytes::<32>());

            // Try to avoid cloning the evm
            let mut evm = self.evm.clone();
            evm.env.tx.data = calldata.into();

            // Call Pow contract to calculate hash
            let result = match evm.transact_preverified().unwrap().result {
                ExecutionResult::Success { output, .. } => match output {
                    Output::Call(out) => out,
                    _ => panic!("EVM Call failed"),
                },
                _ => panic!("Transaction execution failed"),
            };

            // We know the Pow returns bytes32 so we can hardcode the slice
            let output: &[u8; 32] = (&result as &[u8]).try_into().unwrap();

            if std::iter::zip(max_hash, output)
                .take(take_first)
                .all(|(f, s)| s <= &f)
            {
                Some((second_nonce, FixedBytes::<32>::from(output)))
            } else {
                None
            }
        });
        let elapsed = now.elapsed();
        tracing::info!("Mined in {:?}", elapsed);

        result
    }
}
