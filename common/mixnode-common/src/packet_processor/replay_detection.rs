// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};
use fastbloom_rs::{BloomFilter, FilterBuilder, Membership};
use crate::packet_processor::error::MixProcessingError;


const BLOOM_FILTER_SIZE : u64 = 10_000_000;
const FP_RATE : f64 = 1e-4;

//alias for convenience
type ReplayTag = [u8];

#[derive(Clone, Debug)]
pub struct ReplayDetector(Arc<Mutex<ReplayDetectorInner>>);

impl ReplayDetector {
    pub fn new() -> Self {
        ReplayDetector(Arc::new(Mutex::new(
            ReplayDetectorInner::new(),
        )))
    }

    //check if secret has been seen already
    //if yes, return Ok
    //if no, add the secret to the list, then return an error
    pub fn handle_replay_tag(&self, secret : &ReplayTag) -> Result<(), MixProcessingError> {
        match self.0.lock() {
            Ok(mut inner) => {
                if !inner.lookup_then_insert(secret) {
                    Ok(())
                } else {
                    Err(MixProcessingError::ReplayedPacketDetected)
                }
            },
            Err(err) => {
                log::warn!("Failed to handle secret : {err}");
                Ok(()) //what is the sensible thing to do, if the lock is poisoned? Reset the filter ? 
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

impl ReplayDetectorInner {

    pub fn new() -> Self {
        ReplayDetectorInner {
            filter : FilterBuilder::new(BLOOM_FILTER_SIZE, FP_RATE).build_bloom_filter(),
        }
    }

    pub fn lookup_then_insert(&mut self, secret : &ReplayTag) -> bool {
        self.filter.contains_then_add(secret)
    }
}

#[cfg(test)]
mod replay_detector_test {
    use super::*;

    #[test]
    fn handle_replay_tag_correctly_detects_replay() {
        let replay_detector = ReplayDetector::new();
        let secret = b"Hello World!";
        replay_detector.handle_replay_tag(secret);
        assert_eq!(Err(MixProcessingError::ReplayedPacketDetected), replay_detector.handle_replay_tag(secret));
    }

    #[test]
    fn handle_replay_tag_correctly_handle_new_tag() {
        let replay_detector = ReplayDetector::new();
        let secret = b"Hello World!";
        assert_ok!(replay_detector.handle_replay_tag(secret));
    }
}