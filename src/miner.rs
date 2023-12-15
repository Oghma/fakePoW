//! Miner module

use alloy_sol_types::SolCall;
use rayon::prelude::ParallelIterator;
use revm::{
    inspectors::NoOpInspector,
    interpreter::{CallContext, Contract, Interpreter},
    precompile::{Precompiles, SpecId},
    primitives::{
        address, keccak256, AccountInfo, Address, Bytecode, FixedBytes, LatestSpec, Spec,
        TransactTo, U256,
    },
    EVMImpl, InMemoryDB, EVM,
};

use crate::{utils, Pow::mineCall};

pub struct Miner {
    contract_address: Address,
    contract_bytecode: Bytecode,
}

impl Miner {
    pub fn new() -> Self {
        let contract_address = address!("d9145CCE52D386f254917e481eB44e9943F39138");
        // Load contract bytecode
        let contract_bytecode = utils::read_contract();

        Self {
            contract_address,
            contract_bytecode,
        }
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
            .map_init(
                || Worker::new(self.contract_address, self.contract_bytecode.clone()),
                |worker, second_nonce| worker.work(calldata, second_nonce, &max_hash, take_first),
            )
            .find_any(|result| result.is_some());
        let elapsed = now.elapsed();
        tracing::info!("Mined in {:?}", elapsed);

        result.unwrap_or_default()
    }
}

/// Struct for searching the second nonce
///
/// `evm` does many tasks that we are not interested in such as controlling gas
/// and gas limit, validating calldata, addresses etc etc. Futhermore, each
/// iteration create `Contract` and it does some checks that we can ignore. To
/// be as fast as possible, we will use only the revm interpreter.
struct Worker {
    evm: EVM<InMemoryDB>,
    contract: Box<Contract>,
    precompiles: Precompiles,
}

impl Worker {
    pub fn new(contract_address: Address, contract_bytecode: Bytecode) -> Self {
        // Initialise the EVM
        let mut evm = utils::initialise_evm();
        let db = evm.db.as_mut().unwrap();

        // Deploy the contract using a fake address
        let contract_account = AccountInfo::new(
            U256::ZERO,
            0,
            keccak256(&contract_bytecode.bytecode),
            contract_bytecode,
        );
        db.insert_account_info(contract_address, contract_account.clone());

        evm.env.tx.caller = Address::ZERO;
        evm.env.tx.transact_to = TransactTo::Call(contract_address);
        let contract = Box::new(Contract::new_with_context(
            [0 as u8; 1].into(),
            contract_account.code.clone().unwrap(),
            contract_account.code_hash.clone(),
            &CallContext {
                address: contract_address,
                caller: Address::ZERO,
                code_address: contract_address,
                apparent_value: U256::ZERO,
                scheme: revm::interpreter::CallScheme::Call,
            },
        ));

        let precompiles = Precompiles::new(SpecId::from_spec_id(LatestSpec::SPEC_ID)).clone();

        Self {
            evm,
            contract,
            precompiles,
        }
    }

    pub fn work(
        &mut self,
        calldata: [u8; 68],
        second_nonce: U256,
        max_hash: &[u8; 32],
        take_first: usize,
    ) -> Option<(U256, FixedBytes<32>)> {
        // The interpreter requires an environment, a database and an inspector
        // that are created by `EVM`. Thanks friend, I appreciate.
        let mut inspector = NoOpInspector;
        let mut env = self.evm.env.clone();
        let mut db = self.evm.db.as_mut().unwrap();

        // Every call of `evm` creates an `EVMImpl`
        let mut host = EVMImpl::<LatestSpec, InMemoryDB, false>::new(
            &mut db,
            &mut env,
            &mut inspector,
            self.precompiles.clone(),
        );

        let mut calldata = calldata.clone();
        calldata[36..68].copy_from_slice(&second_nonce.to_be_bytes::<32>());
        // Load input
        self.contract.input = calldata.into();

        // Call the interpreter
        let mut interpreter = Box::new(Interpreter::new(self.contract.clone(), u64::MAX, false));
        interpreter.run::<EVMImpl<LatestSpec, InMemoryDB, false>, LatestSpec>(&mut host);

        let output = interpreter.return_value();
        let output: &[u8; 32] = (&output as &[u8]).try_into().unwrap();

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
