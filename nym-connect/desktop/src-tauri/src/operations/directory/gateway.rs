use crate::error::Result;
use nym_api_requests::models::GatewayBondAnnotated;
use nym_contracts_common::types::Percent;

const GATEWAYS_DETAILED_URL: &str = "https://validator.nymtech.net/api/v1/status/gateways/detailed";
const GATEWAY_PERFORMANCE_SCORE_THRESHOLD: u64 = 90;

#[tauri::command]
pub async fn get_gateways() -> Result<Vec<GatewayBondAnnotated>> {
    log::trace!("Fetching gateways");
    let res = reqwest::get(GATEWAYS_DETAILED_URL)
        .await?
        .json::<Vec<GatewayBondAnnotated>>()
        .await?;
    log::trace!("Received: {:#?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn get_gateways_filtered() -> Result<Vec<GatewayBondAnnotated>> {
    let all_gateways = get_gateways().await?;
    let res = all_gateways
        .iter()
        .filter(|g| {
            g.performance
                > Percent::from_percentage_value(GATEWAY_PERFORMANCE_SCORE_THRESHOLD).unwrap()
        })
        .cloned()
        .collect();
    log::trace!("Filtered: {:#?}", res);
    Ok(res)
}
