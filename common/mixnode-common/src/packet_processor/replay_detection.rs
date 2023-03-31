// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};
use fastbloom_rs::{BloomFilter, FilterBuilder, Membership};


const BLOOM_FILTER_SIZE : u64 = 10_000_000;
const FP_RATE : f64 = 1e-4;

//alias for convenience
type GroupElement = [u8];

#[derive(Clone, Debug)]
pub struct ReplayDetector(Arc<Mutex<ReplayDetectorInner>>);

impl ReplayDetector {
    pub fn new() -> Self {
        ReplayDetector(Arc::new(Mutex::new(
            ReplayDetectorInner::new(),
        )))
    }

    //check if secret has been seen already
    //if yes, return True
    //if no, add the secret to the list, then return false
    pub fn handle_secret(&self, secret : &GroupElement) -> bool {
        match self.0.lock() {
            Ok(mut inner) => {
                let seen = inner.lookup(&secret);
                if !seen {
                    inner.insert(secret);
                }
                seen
            },
            Err(err) => {
                log::warn!("Failed to handle secret : {err}");
                false
            }
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
    filter : BloomFilter,
}

impl ReplayDetectorInner { //SW TODO: see if we can lookup and insert at the same time

    pub fn new() -> Self {
        ReplayDetectorInner {
            filter : FilterBuilder::new(BLOOM_FILTER_SIZE, FP_RATE).build_bloom_filter(),
        }
    }
    pub fn lookup(&self, secret : &GroupElement) -> bool {
        self.filter.contains(secret)
    }

    pub fn insert(&mut self, secret : &GroupElement) {
        self.filter.add(secret)
    }
}

#[cfg(test)]
mod replay_detector_test {
    use super::*;

    #[test]
    fn handle_secret_correctly_detects_replay() {
        let replay_detector = ReplayDetector::new();
        let secret = b"Hello World!";
        replay_detector.handle_secret(secret);
        assert!(replay_detector.handle_secret(secret));
    }

    #[test]
    fn handle_secret_correctly_handle_new_secret() {
        let replay_detector = ReplayDetector::new();
        let secret = b"Hello World!";
        assert!(!replay_detector.handle_secret(secret));
    }
}