use itertools::Itertools;

use crate::error::Result;
use crate::models::{
    DirectoryService, DirectoryServiceProvider, HarbourMasterService, PagedResult,
};

// The directory of network requesters
const SERVICE_PROVIDER_WELLKNOWN_URL: &str =
    "https://nymtech.net/.wellknown/connect/service-providers.json";

// Directory of network-requesters running with medium toggle enabled, for testing
const SERVICE_PROVIDER_WELLKNOWN_URL_MEDIUM: &str =
    "https://nymtech.net/.wellknown/connect/service-providers-medium.json";

// Harbour master is used to periodically keep track of which network-requesters are online
const HARBOUR_MASTER_URL: &str = "https://harbourmaster.nymtech.net/v1/services/?size=100";

// We only consdier network requester with a routing score above this threshold
const SERVICE_ROUTING_SCORE_THRESHOLD: f32 = 90.0;

#[tauri::command]
pub async fn get_services() -> Result<Vec<DirectoryServiceProvider>> {
    log::trace!("Fetching services");
    let all_services_with_category = fetch_services().await?;
    log::trace!("Received: {:#?}", all_services_with_category);

    // Flatten all services into a single vector (get rid of categories)
    // We currently don't care about categories, but we might in the future...
    let all_services = all_services_with_category
        .into_iter()
        .flat_map(|sp| sp.items)
        .collect_vec();

    // Early return if we're running with medium toggle enabled
    if std::env::var("NYM_CONNECT_ENABLE_MEDIUM").is_ok() {
        return Ok(all_services);
    }

    // TODO: get paged
    log::trace!("Fetching active services");
    let active_services = fetch_active_services().await?;
    log::trace!("Active: {:#?}", active_services);

    if active_services.items.is_empty() {
        log::warn!("No active services found! Using all services instead, as fallback");
        return Ok(all_services);
    }

    log::trace!("Filter out inactive");
    let filtered_services = filter_out_inactive(&all_services, active_services);
    log::trace!("Filtered out: {:#?}", filtered_services);

    if filtered_services.is_empty() {
        log::warn!(
            "After filtering, active services found! Using all services instead, as fallback"
        );
        return Ok(all_services);
    }

    Ok(filtered_services)
}

fn get_services_url() -> &'static str {
    std::env::var("NYM_CONNECT_ENABLE_MEDIUM")
        .is_ok()
        .then(|| SERVICE_PROVIDER_WELLKNOWN_URL_MEDIUM)
        .unwrap_or(SERVICE_PROVIDER_WELLKNOWN_URL)
}

fn filter_out_inactive(
    all_services: &[DirectoryServiceProvider],
    active_services: PagedResult<HarbourMasterService>,
) -> Vec<DirectoryServiceProvider> {
    // let mut filtered: Vec<DirectoryServiceProvider> = vec![];
    all_services
        .iter()
        .filter(|sp| {
            active_services.items.iter().any(|active| {
                active.service_provider_client_id == sp.address
                    && active.routing_score > SERVICE_ROUTING_SCORE_THRESHOLD
            })
        })
        .cloned()
        .collect()
}

async fn fetch_services() -> Result<Vec<DirectoryService>> {
    let services_url = get_services_url();
    let services_res = reqwest::get(services_url)
        .await?
        .json::<Vec<DirectoryService>>()
        .await?;
    Ok(services_res)
}

async fn fetch_active_services() -> Result<PagedResult<HarbourMasterService>> {
    let active_services = reqwest::get(HARBOUR_MASTER_URL)
        .await?
        .json::<PagedResult<HarbourMasterService>>()
        .await?;
    Ok(active_services)
}
