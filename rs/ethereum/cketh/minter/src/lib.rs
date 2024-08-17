pub mod address;
pub mod blocklist;
mod cbor;
pub mod checked_amount;
pub mod deposit;
pub mod endpoints;
pub mod erc20;
pub mod eth_logs;
pub mod eth_rpc;
pub mod eth_rpc_client;
pub mod eth_rpc_error;
pub mod guard;
pub mod ledger_client;
pub mod lifecycle;
pub mod logs;
pub mod management;
pub mod map;
pub mod memo;
pub mod numeric;
pub mod state;
pub mod storage;
pub mod tx;
pub mod withdraw;

#[cfg(test)]
pub mod test_fixtures;
#[cfg(test)]
mod tests;

use serde_bytes::ByteBuf;
use std::time::Duration;

pub const MAIN_DERIVATION_PATH: Vec<ByteBuf> = vec![];
pub const SCRAPING_ETH_LOGS_INTERVAL: Duration = Duration::from_secs(3 * 60);
pub const PROCESS_ETH_RETRIEVE_TRANSACTIONS_INTERVAL: Duration = Duration::from_secs(6 * 60);
pub const PROCESS_REIMBURSEMENT: Duration = Duration::from_secs(3 * 60);
pub const PROCESS_ETH_RETRIEVE_TRANSACTIONS_RETRY_INTERVAL: Duration = Duration::from_secs(3 * 60);
pub const MINT_RETRY_DELAY: Duration = Duration::from_secs(3 * 60);

// In the following, we register a custom getrandom implementation because
// otherwise getrandom (which is a dependency of k256) fails to compile.
// This is necessary because getrandom by default fails to compile for the
// wasm32-unknown-unknown target (which is required for deploying a canister).
// Our custom implementation always fails, which is sufficient here because
// we only use the k256 crate for verifying secp256k1 signatures, and such
// signature verification does not require any randomness.
getrandom::register_custom_getrandom!(always_fail);
pub fn always_fail(_buf: &mut [u8]) -> Result<(), getrandom::Error> {
    Err(getrandom::Error::UNSUPPORTED)
}
