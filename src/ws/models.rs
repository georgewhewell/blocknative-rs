use chrono::Utc;
// Code adapted from: https://github.com/althea-net/guac_rs/tree/master/web3/src/jsonrpc
// use ethers_core::types::U256;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fmt};
use thiserror::Error;

use crate::models::Blockchain;

#[derive(Serialize, Deserialize, Debug, Clone, Error)]
/// A JSON-RPC 2.0 error
pub struct JsonRpcError {
    /// The error code
    pub code: i64,
    /// The error message
    pub message: String,
    /// Additional data
    pub data: Option<Value>,
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(code: {}, message: {}, data: {:?})",
            self.code, self.message, self.data
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    time_stamp: String,
    dapp_id: String,
    version: String,
    blockchain: Blockchain,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxDescriptor {
    pub hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountDescriptor {
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionSubscribe {
    transaction: TxDescriptor,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountSubscribe {
    account: AccountDescriptor,
}

impl TransactionSubscribe {
    pub fn new(hash: String) -> Self {
        Self {
            transaction: TxDescriptor { hash },
        }
    }
}

impl AccountSubscribe {
    pub fn account(address: String) -> Self {
        Self {
            account: AccountDescriptor { address },
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
/// A JSON-RPC request
#[serde(rename_all = "camelCase")]
pub struct Request<'a, T> {
    #[serde(rename = "timeStamp")]
    timestamp: String,
    dapp_id: &'a str,
    blockchain: Blockchain,
    version: &'a str,
    category_code: String,
    event_code: String,
    #[serde(flatten)]
    params: T,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Subscription<R> {
    pub subscription: u64,
    pub result: R,
}

impl<'a, T> Request<'a, T> {
    /// Creates a new JSON RPC request
    pub fn new(
        dapp_id: &'a str,
        blockchain: Blockchain,
        method: &'a str,
        event_code: &'a str,
        params: T,
    ) -> Self {
        Self {
            timestamp: Utc::now().to_string(),
            dapp_id: dapp_id,
            blockchain,
            version: "2",
            category_code: method.to_string(),
            event_code: event_code.to_string(),
            params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub status: String,
    pub monitor_id: String,
    pub monitor_version: String,
    #[serde(flatten)]
    pub confirmed: Option<ConfirmedInfo>,
    pub pending: Option<PendingInfo>,
    pub hash: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub gas: u64,
    pub nonce: u64,
    pub v: String,
    pub r: String,
    pub s: String,
    pub input: String,
    pub gas_price: String,
    pub gas_price_gwei: u64,
    #[serde(rename = "type")]
    pub type_field: Option<i64>,
    pub asset: String,
    #[serde(flatten)]
    pub watch_info: Option<WatchedAddressInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingInfo {
    pub pending_time_stamp: String,
    pub pending_block_number: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedInfo {
    pub time_pending: String,
    pub blocks_pending: i64,
    pub block_hash: String,
    pub block_number: i64,
    pub transaction_index: i64,
    pub block_time_stamp: String,
    pub gas_used: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedAddressInfo {
    pub watched_address: String,
    pub direction: String,
    pub counterparty: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractCall {
    pub contract_type: String,
    pub contract_address: String,
    pub method_name: String,
    pub params: Value,
    pub contract_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub time_stamp: String,
    pub category_code: String,
    pub event_code: String,
    pub dapp_id: String,
    pub blockchain: Blockchain,
    pub contract_call: Option<ContractCall>,
    pub transaction: Option<Transaction>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub version: u64,
    pub server_version: String,
    pub time_stamp: String,
    pub connection_id: String,
    pub status: String,
    pub raw: Option<String>,
    pub event: Option<Event>,
    pub reason: Option<String>,
    pub dispatch_timestamp: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelloMsg {
    pub version: i64,
    pub server_version: String,
    pub status: String,
    #[serde(rename = "showUX")]
    pub show_ux: bool,
    pub connection_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchConfig {
    pub scope: String,
    pub filters: Vec<HashMap<String, String>>,
    pub abi: Vec<Value>,
    pub watch_address: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchRequest {
    pub config: WatchConfig,
}
