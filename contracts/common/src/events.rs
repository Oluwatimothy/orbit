//! Orbit-specific NEP-297 events. Standard token and NFT events come from
//! `near-contract-standards`.

use near_sdk::env;
use near_sdk::serde::Serialize;
use near_sdk::serde_json::json;

pub const EVENT_STANDARD: &str = "orbit";
pub const EVENT_VERSION: &str = "1.0.0";

/// Logs `EVENT_JSON:{"standard":"orbit","version":"1.0.0","event":…,"data":[…]}`.
pub fn emit<T: Serialize>(event: &str, data: &T) {
    let payload = json!({
        "standard": EVENT_STANDARD,
        "version": EVENT_VERSION,
        "event": event,
        "data": [data],
    });
    env::log_str(&format!("EVENT_JSON:{payload}"));
}
