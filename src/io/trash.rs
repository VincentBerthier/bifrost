use crate::crypto::Pubkey;

use super::location::AccountDiskLocation;

#[derive(Default)]
pub struct Trash {
    trash: Vec<(Pubkey, AccountDiskLocation)>,
}

impl Trash {
    pub fn insert(&mut self, key: Pubkey, loc: AccountDiskLocation) {
        self.trash.push((key, loc));
    }

    pub fn len(&self) -> usize {
        self.trash.len()
    }
}
