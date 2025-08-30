#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

#[macro_use]
extern crate alloc;

use alloc::vec::Vec;
use stylus_sdk::{alloy_primitives::U256, prelude::*};

sol_storage! {
    #[entrypoint]
    pub struct Counter {
        uint256 red_swords;
        uint256 green_swords;
        uint256 blue_swords;
    }
}

#[public]
impl Counter {
    pub fn get_sword_counts(&self) -> (U256, U256, U256) {
        (self.red_swords.get(), self.green_swords.get(), self.blue_swords.get())
    }

    pub fn increment_sword(&mut self, color: U256) {
        if color == U256::from(0) {
            self.red_swords.set(self.red_swords.get() + U256::from(1));
        } else if color == U256::from(1) {
            self.green_swords.set(self.green_swords.get() + U256::from(1));
        } else if color == U256::from(2) {
            self.blue_swords.set(self.blue_swords.get() + U256::from(1));
        }
    }
}