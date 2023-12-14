//! Module containing utilities functions

use rayon::{
    iter::plumbing::{bridge_unindexed, UnindexedProducer},
    prelude::ParallelIterator,
};
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{hex, Bytecode},
    InMemoryDB, EVM,
};
use ruint::{aliases::U256, uint};

/// Load from file `Pow` bytecode
pub fn read_contract() -> Bytecode {
    //let tmp = fs::read_to_string("contracts/Pow.bin").expect("Invalid `Pow.bin` location");
    //
    // For now hardcode the bytecode. If we want read from file we need to
    // remove the constructor or deploy it in revm
    Bytecode::new_raw("6080604052348015600e575f80fd5b50600436106026575f3560e01c8063071e950314602a575b5f80fd5b606160353660046073565b604080516020808201949094528082019290925280518083038201815260609092019052805191012090565b60405190815260200160405180910390f35b5f80604083850312156083575f80fd5b5050803592602090910135915056fea26469706673582212201676b931d82af5bbf61cc03592b8a3e8c28dac7cdf08deae042e43adf84b041264736f6c63430008160033".parse().unwrap())

    //Bytecode::new_raw("0x6080604052348015600e575f80fd5b50600436106026575f3560e01c8063c0e0b3dd14602a575b5f80fd5b606a60353660046093565b6040805163ffffffff938416602080830191909152929093168382015280518084038201815260609093019052815191012090565b60405190815260200160405180910390f35b803563ffffffff81168114608e575f80fd5b919050565b5f806040838503121560a3575f80fd5b60aa83607c565b915060b660208401607c565b9050925092905056fea26469706673582212209628074345e5b207dd701f8447c31c4003b03fd53c62a974b9c315c1a9d8e28364736f6c63430008140033".parse().unwrap())
}

/// Return a new EVM
pub fn initialise_evm() -> EVM<InMemoryDB> {
    let db = CacheDB::new(EmptyDB::default());
    let mut evm = EVM::new();
    evm.database(db);

    evm
}

/// Return a [u8;32] containing `num` leading zeros
pub fn num_0s(num: usize) -> [u8; 32] {
    let mut output: [u8; 32] = [0; 32];
    let zeros = "0".repeat(num);
    let fs = "f".repeat(64 - num);

    hex::decode_to_slice(zeros + &fs, &mut output).unwrap();
    output
}

pub struct UintRange {
    start: U256,
    end: U256,
}

impl UintRange {
    pub fn new(start: U256, end: U256) -> Self {
        UintRange { start, end }
    }

    pub fn len(&self) -> U256 {
        self.end - self.start
    }
}

impl Iterator for UintRange {
    type Item = U256;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            let result = Some(self.start);
            self.start += uint!(1_U256);
            result
        }
    }
}

impl ParallelIterator for UintRange {
    type Item = U256;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        bridge_unindexed(UintProducer { range: self }, consumer)
    }

    fn opt_len(&self) -> Option<usize> {
        usize::try_from(self.len()).ok()
    }
}

struct UintProducer {
    range: UintRange,
}

impl IntoIterator for UintProducer {
    type Item = <UintRange as Iterator>::Item;
    type IntoIter = UintRange;

    fn into_iter(self) -> Self::IntoIter {
        self.range
    }
}

impl UnindexedProducer for UintProducer {
    type Item = U256;

    fn split(mut self) -> (Self, Option<Self>) {
        let index = self.range.len().div_ceil(uint!(2_U256));
        if index > U256::ZERO {
            let mid = self.range.start + index;
            let right = UintRange::new(mid, self.range.end);
            self.range.end = mid;
            (self, Some(UintProducer { range: right }))
        } else {
            (self, None)
        }
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: rayon::iter::plumbing::Folder<Self::Item>,
    {
        folder.consume_iter(self)
    }
}
