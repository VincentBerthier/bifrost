use borsh::{BorshDeserialize, BorshSerialize};

/// A wallet as saved on the chain
#[derive(Copy, Clone, Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct Wallet {
    /// Number of prisms on the wallet.
    pub prisms: u64,
}
