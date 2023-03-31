// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, RwLock};
use fastbloom_rs::{BloomFilter, FilterBuilder, Membership};


const BLOOM_FILTER_SIZE : u64 = 10_000_000;
const FP_RATE : f64 = 1e-4;

//alias for convenience
type GroupElement = [u8];

#[derive(Clone, Debug)]
pub struct ReplayDetector(Arc<RwLock<ReplayDetectorInner>>);

impl ReplayDetector {
    pub fn new() -> Self {
        ReplayDetector(Arc::new(RwLock::new(
            ReplayDetectorInner::new(),
        )))
    }

    //check if secret has been seen already
    //if yes, return True
    //if no, add the secret to the list, then return false
    pub fn handle_secret(&self, secret : &GroupElement) -> bool {
        let seen = self.lookup(secret);
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

    pub fn insert(&self, secret : &GroupElement) {
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
    fn lookup_after_insert_returns_true() {
        let replay_detector = ReplayDetector::new();
        let secret = b"Hello World!";
        replay_detector.insert(secret);
        assert!(replay_detector.lookup(secret));
    }

    #[test]
    fn lookup_new_value_returns_false() {
        let replay_detector = ReplayDetector::new();
        let secret = b"Hello World!";
        assert!(!replay_detector.lookup(secret));
    }

    #[test]
    fn handle_secret_correctly_detects_replay() {
        let replay_detector = ReplayDetector::new();
        let secret = b"Hello World!";
        replay_detector.insert(secret);
        assert!(replay_detector.handle_secret(secret));
    }

    #[test]
    fn handle_secret_correctly_handle_new_secret() {
        let replay_detector = ReplayDetector::new();
        let secret = b"Hello World!";
        assert!(!replay_detector.handle_secret(secret));
        assert!(replay_detector.lookup(secret));
    }
}