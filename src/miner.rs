//! Miner module

use alloy_sol_types::SolCall;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use revm::{
    primitives::{
        address, keccak256, AccountInfo, Address, Bytecode, ExecutionResult, FixedBytes, Output,
        TransactTo, U256,
    },
    InMemoryDB, EVM,
};

use crate::{utils, Pow::mineCall};

pub struct Miner {
    worker: Worker,
}

impl Miner {
    pub fn new() -> Self {
        let contract_address = address!("d9145CCE52D386f254917e481eB44e9943F39138");
        // Load contract bytecode
        let contract_bytecode = utils::read_contract();
        let worker = Worker::new(contract_address, contract_bytecode);

        Self { worker }
    }

    /// Search for a valid hash
    pub fn mine(&self, leading_zeros: usize, first_nonce: U256) -> Option<(U256, FixedBytes<32>)> {
        let max_hash = utils::num_0s(leading_zeros);
        let take_first = (leading_zeros + (leading_zeros & 1)) / 2;
        let range = utils::UintRange::new(U256::ZERO, U256::MAX);

        // First 36 bytes are fixed, prefetch it
        let mut calldata: [u8; 68] = [0; 68];
        calldata[..4].copy_from_slice(&mineCall::SELECTOR);
        calldata[4..36].copy_from_slice(&first_nonce.to_be_bytes::<32>());

        let now = std::time::Instant::now();
        let result = range
            .into_par_iter()
            .map_with(self.worker.clone(), |worker, second_nonce| {
                worker.work(calldata, second_nonce, &max_hash, take_first)
            })
            .find_any(|result| result.is_some());
        let elapsed = now.elapsed();
        tracing::info!("Mined in {:?}", elapsed);

        result.unwrap_or_default()
    }
}

#[derive(Clone)]
struct Worker {
    evm: EVM<InMemoryDB>,
}

impl Worker {
    pub fn new(contract_address: Address, contract_bytecode: Bytecode) -> Self {
        // Initialise the EVM
        let mut evm = utils::initialise_evm();
        let db = evm.db.as_mut().unwrap();

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

    pub fn work(
        &mut self,
        calldata: [u8; 68],
        second_nonce: U256,
        max_hash: &[u8; 32],
        take_first: usize,
    ) -> Option<(U256, FixedBytes<32>)> {
        let mut calldata = calldata.clone();
        calldata[36..68].copy_from_slice(&second_nonce.to_be_bytes::<32>());

        self.evm.env.tx.data = calldata.into();
        // Call Pow contract to calculate hash
        let result = match self.evm.transact_preverified().unwrap().result {
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
    }
}
