/*!
 * This module implements services related to accounts.
 */

use std::collections::HashMap;
use std::iter::Iterator;

use super::model::*;


/// Storage service for client accounts.
pub struct AccountStorage {
    /// The store uses a hash map for fast access to accounts.
    accounts: HashMap<ClientId, Account>,
}

impl AccountStorage {
    /// Creates an empty account storage.
    pub fn new() -> Self {
        AccountStorage {
            accounts: HashMap::new(),
        }
    }

    /// Gets a client's account, creating one if it doesn't exist yet.
    fn get_client_account<'a, 'b>(&'a mut self, client_id: &'b ClientId) -> &'a mut Account {
        if !self.accounts.contains_key(client_id) {
            let acc = Account::new(*client_id);
            self.accounts.insert(*client_id, acc);
        }

        self.accounts.get_mut(client_id).unwrap()
    }

    /// Apply a single transaction to the correct client account.
    pub fn apply_transaction(&mut self, client_id: &ClientId, tx: Transaction) -> Result<()> {
        let acc = self.get_client_account(client_id);
        acc.apply(tx)
    }

    /// Return an iterator over all user accounts.
    pub fn iter(&self) -> AccountStorageIter {
        AccountStorageIter { iter: self.accounts.iter() }
    }
}


pub struct AccountStorageIter<'a> {
    iter: std::collections::hash_map::Iter<'a, ClientId, Account>,
}

impl<'a> Iterator for AccountStorageIter<'a> {
    type Item = (&'a ClientId, &'a Account);

    fn next(&mut self) -> Option<(&'a ClientId, &'a Account)> {
        self.iter.next()
    }
}
