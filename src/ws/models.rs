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
    // Creates a new JSON RPC request
    pub fn new(
        dapp_id: &'a str,
        blockchain: Blockchain,
        method: &'a str,
        event_code: &'a str,
        params: T,
    ) -> Self {
        Self {
            timestamp: Utc::now().to_string(),
            dapp_id,
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

#[cfg(feature = "ethers")]
impl From<Transaction> for ethers::prelude::Transaction {
    fn from(val: Transaction) -> Self {
        ethers::prelude::Transaction {
            from: val.from.parse().unwrap(),
            to: Some(val.to.parse().unwrap()),
            gas: val.gas.into(),
            gas_price: Some(val.gas_price.parse().unwrap()),
            value: val.value.parse().unwrap(),
            nonce: val.nonce.into(),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            input: hex::decode(val.input.strip_prefix("0x").unwrap())
                .unwrap()
                .into(),
            v: val.v.parse().unwrap(),
            r: val.r.parse().unwrap(),
            s: val.s.parse().unwrap(),
            transaction_type: val.type_field.map(|n| n.into()),
            access_list: None,
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            chain_id: None,
            hash: val.hash.parse().unwrap(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        let json = r#"{"version":0,"serverVersion":"0.123.2","timeStamp":"2021-12-07T10:20:25.212Z","connectionId":"C4-bc4de41f-c42f-460a-af83-28ad95286ab0","status":"ok","event":{"timeStamp":"2021-12-07T10:20:25.212Z","categoryCode":"activeAddress","eventCode":"txConfirmed","dappId":"7d507b2c-48f2-48bb-bd79-fc16ced6f8cf","blockchain":{"system":"ethereum","network":"matic-main"},"contractCall":{"contractType":"Uniswap V2: Router 2","contractAddress":"0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff","methodName":"swapExactTokensForTokens","params":{"amountIn":"5000000000","amountOutMin":"180189367","path":["0xC250e9987A032ACAC293d838726C511E6E1C029d","0xa3Fa99A148fA48D14Ed51d610c367C61876997F1","0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174","0xc2132D05D31c914a87C6611C10748AEb04B58e8F"],"to":"0x21F3bB63e775ccDf0CC04559Be142971D241aB0E","deadline":"3277746025"},"contractName":"QuickSwap: Router"},"transaction":{"status":"confirmed","monitorId":"Geth_137_C_PROD","monitorVersion":"0.102.0","timePending":"3146","blocksPending":3,"pendingTimeStamp":"2021-12-07T10:20:22.066Z","pendingBlockNumber":22235980,"hash":"0xe0b1cf2bea578f49ba78cacd0d12d9c013f07cdd987936e71965edf6bd972b78","from":"0x21F3bB63e775ccDf0CC04559Be142971D241aB0E","to":"0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff","value":"0","gas":387473,"nonce":45,"blockHash":"0xa814777d863e89c2b565ad4947e37e48bc5d8407b4065303c6371de519980d89","blockNumber":22235983,"v":"0x136","r":"0xb1fa90713d69a05869823607cc4bc67de6c7d4599b9fe8b00c54d8bc902739f9","s":"0x297a6aba5a47be29475d037b41619ad4003048e82305f20a3b18927cbfe2a343","input":"0x38ed1739000000000000000000000000000000000000000000000000000000012a05f200000000000000000000000000000000000000000000000000000000000abd78b700000000000000000000000000000000000000000000000000000000000000a000000000000000000000000021f3bb63e775ccdf0cc04559be142971d241ab0e00000000000000000000000000000000000000000000000000000000c35e6f690000000000000000000000000000000000000000000000000000000000000004000000000000000000000000c250e9987a032acac293d838726c511e6e1c029d000000000000000000000000a3fa99a148fa48d14ed51d610c367c61876997f10000000000000000000000002791bca1f2de4661ed88a30c99a7a9449aa84174000000000000000000000000c2132d05d31c914a87c6611c10748aeb04b58e8f","gasPrice":"113000000000","gasPriceGwei":113,"gasUsed":"236672","transactionIndex":1,"asset":"","blockTimeStamp":"2021-12-07T10:20:25.000Z","watchedAddress":"0xa5e0829caced8ffdd4de3c43696c57f7d7a678ff","direction":"incoming","counterparty":"0x21F3bB63e775ccDf0CC04559Be142971D241aB0E"}},"dispatchTimestamp":"2021-12-07T10:20:25.247Z"}"#;
        let resp: Response = serde_json::from_str(json).unwrap();
    }
}
