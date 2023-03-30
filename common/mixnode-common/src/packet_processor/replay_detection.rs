// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, RwLock};


//alias for convenience
type GroupElement = Vec<u8>;

#[derive(Clone, Debug)]
pub struct ReplayDetector(Arc<RwLock<ReplayDetectorInner>>);

impl ReplayDetector {
    pub fn new() -> Self {
        ReplayDetector(Arc::new(RwLock::new(
            ReplayDetectorInner{
                set : Vec::new(),
            }
        )))
    }

    //check if secret has been seen already
    //if yes, return True
    //if no, add the secret to the list, then return false
    pub fn handle_secret(&self, secret : GroupElement) -> bool {
        let seen = self.lookup(&secret);
        if !seen {
            self.insert(secret);
        }
        seen
    }

    pub fn lookup(&self, secret : &GroupElement) -> bool {
        match self.0.read() {
            Ok(inner) => {
                inner.lookup(secret)
            }
            Err(err) => {
                log::warn!("Failed to lookup secret : {err}");
                false
            }
        }
    }

    pub fn insert(&self, secret : GroupElement) {
        match self.0.write() {
            Ok(mut inner) => inner.insert(secret),
            Err(err) => log::warn!("Failed to insert secret : {err}")
        }
    }

}

impl Default for ReplayDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct ReplayDetectorInner {
    set : Vec<GroupElement>
}

impl ReplayDetectorInner {
    pub fn lookup(&self, secret : &GroupElement) -> bool {
        self.set.contains(secret)
    }

    pub fn insert(&mut self, secret : GroupElement) {
        self.set.push(secret)
    }
}