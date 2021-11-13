use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum System {
    Ethereum,
    Bitcoin,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Main,
    Ropsten,
    Rinkeby,
    Goerli,
    Kovan,
    XDai,
    #[serde(rename = "bsc-main")]
    BSC,
    #[serde(rename = "matic-main")]
    Polygon,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Blockchain {
    pub system: System,
    pub network: Network,
}
