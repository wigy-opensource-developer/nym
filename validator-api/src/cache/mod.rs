// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd_client::Client;
use anyhow::Result;
use config::defaults::VALIDATOR_API_VERSION;
use mixnet_contract::{GatewayBond, MixNodeBond, StateParams};
use rocket::fairing::AdHoc;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time;
use validator_client::nymd::CosmWasmClient;

pub(crate) mod routes;

pub struct ValidatorCacheRefresher<C> {
    nymd_client: Client<C>,
    cache: ValidatorCache,
    caching_interval: Duration,
}

#[derive(Clone)]
pub struct ValidatorCache {
    inner: Arc<ValidatorCacheInner>,
}

struct ValidatorCacheInner {
    initialised: AtomicBool,
    mixnodes: RwLock<Cache<Vec<MixNodeBond>>>,
    gateways: RwLock<Cache<Vec<GatewayBond>>>,

    active_mixnodes_available: AtomicBool,
    active_gateways_available: AtomicBool,

    current_mixnode_active_set_size: AtomicUsize,
    current_gateway_active_set_size: AtomicUsize,

    total_mix_stake: RwLock<Cache<u128>>,
    total_gateway_stake: RwLock<Cache<u128>>,
}

#[derive(Default, Serialize, Clone)]
pub struct Cache<T> {
    value: T,
    as_at: u64,
}

impl<T: Clone> Cache<T> {
    fn set(&mut self, value: T) {
        self.value = value;
        self.as_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<C> ValidatorCacheRefresher<C> {
    pub(crate) fn new(
        nymd_client: Client<C>,
        caching_interval: Duration,
        cache: ValidatorCache,
    ) -> Self {
        ValidatorCacheRefresher {
            nymd_client,
            cache,
            caching_interval,
        }
    }

    async fn refresh_cache(&self) -> Result<()>
    where
        C: CosmWasmClient + Sync,
    {
        let (mixnodes, gateways, total_mix_stake, total_gt_state) = tokio::try_join!(
            self.nymd_client.get_mixnodes(),
            self.nymd_client.get_gateways(),
            self.nymd_client.get_total_mix_stake(),
            self.nymd_client.get_total_gateway_stake()
        )?;

        let state_params = self.nymd_client.get_state_params().await?;

        info!(
            "Updating validator cache. There are {} mixnodes and {} gateways, total_mix_stake is {}",
            mixnodes.len(),
            gateways.len(),
            total_mix_stake
        );

        self.cache
            .update_cache(
                mixnodes,
                gateways,
                total_mix_stake,
                total_gt_state,
                state_params,
            )
            .await;

        Ok(())
    }

    pub(crate) async fn run(&self)
    where
        C: CosmWasmClient + Sync,
    {
        let mut interval = time::interval(self.caching_interval);
        loop {
            interval.tick().await;
            if let Err(err) = self.refresh_cache().await {
                error!("Failed to refresh validator cache - {}", err);
            } else {
                // relaxed memory ordering is fine here. worst case scenario network monitor
                // will just have to wait for an additional backoff to see the change.
                // And so this will not really incur any performance penalties by setting it every loop iteration
                self.cache.inner.initialised.store(true, Ordering::Relaxed)
            }
        }
    }
}

impl ValidatorCache {
    fn new() -> Self {
        ValidatorCache {
            inner: Arc::new(ValidatorCacheInner::new()),
        }
    }

    pub fn stage() -> AdHoc {
        AdHoc::on_ignite("Validator Cache Stage", |rocket| async {
            rocket.manage(Self::new()).mount(
                // this format! is so ugly...
                format!("/{}", VALIDATOR_API_VERSION),
                routes![
                    routes::get_mixnodes,
                    routes::get_gateways,
                    routes::get_active_mixnodes,
                    routes::get_active_gateways
                ],
            )
        })
    }

    // TODO: check if all nodes can be compared together,
    // i.e. they all have the same denom for bonds and delegations
    fn verify_mixnodes(&self, mixnodes: &[MixNodeBond]) -> bool {
        if mixnodes.is_empty() {
            return true;
        }
        let expected_denom = &mixnodes[0].bond_amount.denom;
        for mixnode in mixnodes {
            if &mixnode.bond_amount.denom != expected_denom
                || &mixnode.total_delegation.denom != expected_denom
            {
                return false;
            }
        }

        true
    }

    // TODO: check if all nodes can be compared together,
    // i.e. they all have the same denom for bonds and delegations
    fn verify_gateways(&self, gateways: &[GatewayBond]) -> bool {
        if gateways.is_empty() {
            return true;
        }
        let expected_denom = &gateways[0].bond_amount.denom;
        for gateway in gateways {
            if &gateway.bond_amount.denom != expected_denom
                || &gateway.total_delegation.denom != expected_denom
            {
                return false;
            }
        }

        true
    }

    async fn update_cache(
        &self,
        mut mixnodes: Vec<MixNodeBond>,
        mut gateways: Vec<GatewayBond>,
        total_mix_stake: u128,
        total_gt_stake: u128,
        state: StateParams,
    ) {
        // if our data is valid, it means the active sets are available,
        // otherwise we must explicitly indicate nobody can read this data

        if self.verify_mixnodes(&mixnodes) {
            // partial_cmp can only fail if the nodes have different denomination,
            // but we just checked for that hence the unwraps are fine here
            // Note the reverse order of comparison so that the "highest" node would be first
            mixnodes.sort_by(|a, b| b.partial_cmp(a).unwrap());
            self.inner
                .active_mixnodes_available
                .store(true, Ordering::SeqCst);
            self.inner
                .current_mixnode_active_set_size
                .store(state.mixnode_active_set_size as usize, Ordering::SeqCst);
        } else {
            self.inner
                .active_mixnodes_available
                .store(false, Ordering::SeqCst);
        }

        if self.verify_gateways(&gateways) {
            // partial_cmp can only fail if the nodes have different denomination,
            // but we just checked for that hence the unwraps are fine here
            // Note the reverse order of comparison so that the "highest" node would be first
            gateways.sort_by(|a, b| b.partial_cmp(a).unwrap());
            self.inner
                .active_gateways_available
                .store(true, Ordering::SeqCst);
            self.inner
                .current_gateway_active_set_size
                .store(state.gateway_active_set_size as usize, Ordering::SeqCst);
        } else {
            self.inner
                .active_gateways_available
                .store(false, Ordering::SeqCst);
        }

        self.inner.mixnodes.write().await.set(mixnodes);
        self.inner.gateways.write().await.set(gateways);
        self.inner
            .total_mix_stake
            .write()
            .await
            .set(total_mix_stake);
        self.inner
            .total_gateway_stake
            .write()
            .await
            .set(total_gt_stake);
    }

    pub async fn mixnodes(&self) -> Cache<Vec<MixNodeBond>> {
        self.inner.mixnodes.read().await.clone()
    }

    pub async fn gateways(&self) -> Cache<Vec<GatewayBond>> {
        self.inner.gateways.read().await.clone()
    }

    pub async fn active_mixnodes(&self) -> Option<Cache<Vec<MixNodeBond>>> {
        // if active set is available, it means it is already sorted
        if self.inner.active_mixnodes_available.load(Ordering::SeqCst) {
            let cache = self.inner.mixnodes.read().await;
            let timestamp = cache.as_at;
            let nodes = cache
                .value
                .iter()
                .take(
                    self.inner
                        .current_mixnode_active_set_size
                        .load(Ordering::SeqCst),
                )
                .cloned()
                .collect();
            Some(Cache {
                value: nodes,
                as_at: timestamp,
            })
        } else {
            None
        }
    }

    pub async fn active_gateways(&self) -> Option<Cache<Vec<GatewayBond>>> {
        // if active set is available, it means it is already sorted
        if self.inner.active_gateways_available.load(Ordering::SeqCst) {
            let cache = self.inner.gateways.read().await;
            let timestamp = cache.as_at;
            let nodes = cache
                .value
                .iter()
                .take(
                    self.inner
                        .current_gateway_active_set_size
                        .load(Ordering::SeqCst),
                )
                .cloned()
                .collect();
            Some(Cache {
                value: nodes,
                as_at: timestamp,
            })
        } else {
            None
        }
    }

    pub fn initialised(&self) -> bool {
        self.inner.initialised.load(Ordering::Relaxed)
    }

    pub(crate) async fn wait_for_initial_values(&self) {
        let initialisation_backoff = Duration::from_secs(5);
        loop {
            if self.initialised() {
                break;
            } else {
                debug!("Validator cache hasn't been initialised yet - waiting for {:?} before trying again", initialisation_backoff);
                tokio::time::sleep(initialisation_backoff).await;
            }
        }
    }
}

impl ValidatorCacheInner {
    fn new() -> Self {
        ValidatorCacheInner {
            initialised: AtomicBool::new(false),
            mixnodes: RwLock::new(Cache::default()),
            gateways: RwLock::new(Cache::default()),
            active_mixnodes_available: AtomicBool::new(false),
            active_gateways_available: AtomicBool::new(false),
            current_mixnode_active_set_size: Default::default(),
            current_gateway_active_set_size: Default::default(),
            total_mix_stake: Default::default(),
            total_gateway_stake: Default::default(),
        }
    }
}
