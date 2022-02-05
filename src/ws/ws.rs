use futures_channel::{mpsc, oneshot};
use futures_util::{
    sink::{Sink, SinkExt},
    stream::{Fuse, Stream, StreamExt},
};
use serde::Serialize;
use std::{
    collections::{btree_map::Entry, BTreeMap},
    fmt::{self, Debug},
};
use thiserror::Error;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, protocol::CloseFrame},
};

use super::models::{
    HelloMsg, JsonRpcError, Request, Response, TransactionSubscribe, WatchConfig, WatchRequest,
};
use crate::models::Blockchain;
use tracing::{debug, error, warn};

type Pending = oneshot::Sender<Result<serde_json::Value, JsonRpcError>>;
type Subscription = mpsc::UnboundedSender<Response>;

type Message = tungstenite::protocol::Message;
type WsError = tungstenite::Error;
type WsStreamItem = Result<Message, WsError>;

/// Instructions for the `WsServer`.
#[derive(Debug)]
enum Instruction {
    // Send keepalive
    Ping,
    /// JSON-RPC request
    Request {
        request: String,
    },
    /// Create a new subscription
    Subscribe {
        id: u64,
        sink: Subscription,
    },
    /// Cancel an existing subscription
    Unsubscribe {
        id: u64,
    },
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum Incoming {
    HelloMsg(HelloMsg),
    Response(Response),
}

/// Client over Websockets.
#[derive(Clone)]
pub struct Ws {
    instructions: mpsc::UnboundedSender<Instruction>,
    api_key: String,
    blockchain: Blockchain,
}

impl Debug for Ws {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebsocketProvider").finish()
    }
}

impl Ws {
    /// Initializes a new WebSocket Client, given a Stream/Sink Websocket implementer.
    /// The websocket connection must be initiated separately.
    pub fn new<S: 'static>(ws: S, api_key: &str, blockchain: Blockchain) -> Self
    where
        S: Send + Sync + Stream<Item = WsStreamItem> + Sink<Message, Error = WsError> + Unpin,
    {
        let (sink, stream) = mpsc::unbounded();

        let mut ping_sink = sink.clone();
        tokio::task::spawn(async move {
            loop {
                ping_sink.send(Instruction::Ping).await.unwrap();
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });

        // Spawn the server
        WsServer::new(ws, stream).spawn();

        Self {
            blockchain,
            instructions: sink,
            api_key: api_key.to_string(),
        }
    }

    /// Returns true if the WS connection is active, false otherwise
    pub fn ready(&self) -> bool {
        !self.instructions.is_closed()
    }

    /// Initializes a new WebSocket Client
    pub async fn connect(
        url: impl tungstenite::client::IntoClientRequest + Unpin,
        api_key: &str,
        blockchain: Blockchain,
    ) -> Result<Self, ClientError> {
        let (ws, _) = connect_async(url).await?;
        let me = Self::new(ws, api_key, blockchain);
        me.cast("initialize", "checkDappId", ()).await.unwrap();
        Ok(me)
    }

    fn send(&self, msg: Instruction) -> Result<(), ClientError> {
        self.instructions
            .unbounded_send(msg)
            .map_err(to_client_error)
    }

    // type Error = ClientError;
    async fn cast<T: Serialize + Send + Sync>(
        &self,
        method: &str,
        code: &str,
        params: T,
    ) -> Result<(), ClientError> {
        // send the message
        let payload = Instruction::Request {
            request: serde_json::to_string(&Request::new(
                &self.api_key.to_string(),
                self.blockchain.clone(),
                method,
                code,
                params,
            ))?,
        };

        // send the data
        self.send(payload)?;

        Ok(())
    }
}
pub type NotificationStream = mpsc::UnboundedReceiver<Response>;

impl Ws {
    pub async fn listen(&self, config: WatchConfig) -> Result<NotificationStream, ClientError> {
        let (sink, stream) = mpsc::unbounded();

        tracing::info!("Subscribing to filter on scope: {}", config.scope);

        let req = WatchRequest { config };
        self.cast("configs", "put", req).await.unwrap();

        self.send(Instruction::Subscribe {
            id: 1u32.into(),
            sink,
        })?;

        Ok(stream)
    }

    pub async fn unsubscribe<T: Into<u64>>(&self, id: T) -> Result<(), ClientError> {
        self.cast(
            "activeTransaction",
            "unwatch",
            TransactionSubscribe::new(
                "0x0b4c94c414f71ddd5e7a625fcaa83ff1f93e9a7ca37e0f577b488ac8fd786655".to_string(),
            ),
        )
        .await
        .unwrap();
        self.send(Instruction::Unsubscribe { id: id.into() })
    }
}

struct WsServer<S> {
    ws: Fuse<S>,
    instructions: Fuse<mpsc::UnboundedReceiver<Instruction>>,
    pending: Vec<Pending>,
    subscriptions: BTreeMap<u64, Subscription>,
}

impl<S> WsServer<S>
where
    S: Send + Sync + Stream<Item = WsStreamItem> + Sink<Message, Error = WsError> + Unpin,
{
    /// Instantiates the Websocket Server
    fn new(ws: S, requests: mpsc::UnboundedReceiver<Instruction>) -> Self {
        Self {
            // Fuse the 2 steams together, so that we can `select` them in the
            // Stream implementation
            ws: ws.fuse(),
            instructions: requests.fuse(),
            pending: Vec::default(),
            subscriptions: BTreeMap::default(),
        }
    }

    /// Returns whether the all work has been completed.
    ///
    /// If this method returns `true`, then the `instructions` channel has been closed and all
    /// pending requests and subscriptions have been completed.
    fn is_done(&self) -> bool {
        self.instructions.is_done() && self.pending.is_empty() && self.subscriptions.is_empty()
    }

    /// Spawns the event loop
    fn spawn(mut self)
    where
        S: 'static,
    {
        let f = async move {
            loop {
                if self.is_done() {
                    debug!("work complete");
                    break;
                }
                match self.tick().await {
                    Err(ClientError::UnexpectedClose) => {
                        error!("{}", ClientError::UnexpectedClose);
                        break;
                    }
                    Err(e) => {
                        panic!("WS Server panic: {}", e);
                    }
                    _ => {}
                }
            }
        };

        tokio::spawn(f);
    }

    // dispatch an RPC request
    async fn service_request(&mut self, request: String) -> Result<(), ClientError> {
        tracing::debug!("Sending to ws: {:#?}", &request);
        if let Err(e) = self.ws.send(Message::Text(request)).await {
            error!("WS connection error: {:?}", e);
            self.pending.pop();
        }

        Ok(())
    }

    /// Dispatch a subscription request
    async fn service_ping(&mut self) -> Result<(), ClientError> {
        self.ws.send(Message::Ping(vec![])).await?;
        Ok(())
    }

    /// Dispatch a subscription request
    async fn service_subscribe(&mut self, id: u64, sink: Subscription) -> Result<(), ClientError> {
        if self.subscriptions.insert(id, sink).is_some() {
            warn!("Replacing already-registered subscription with id {:?}", id);
        } else {
        }
        // self.service_request(request)
        Ok(())
    }

    /// Dispatch a unsubscribe request
    async fn service_unsubscribe(&mut self, id: u64) -> Result<(), ClientError> {
        if self.subscriptions.remove(&id).is_none() {
            warn!(
                "Unsubscribing from non-existent subscription with id {:?}",
                id
            );
        }
        Ok(())
    }

    /// Dispatch an outgoing message
    async fn service(&mut self, instruction: Instruction) -> Result<(), ClientError> {
        match instruction {
            Instruction::Request {
                // id,
                request,
                // sender,
            } => self.service_request(request).await,
            Instruction::Ping => self.service_ping().await,
            Instruction::Subscribe { id, sink } => self.service_subscribe(id, sink).await,
            Instruction::Unsubscribe { id } => self.service_unsubscribe(id).await,
        }
    }

    async fn handle_ping(&mut self, inner: Vec<u8>) -> Result<(), ClientError> {
        tracing::debug!("handle ping: {:?}", inner);
        self.ws.send(Message::Pong(inner)).await?;
        Ok(())
    }

    async fn handle_text(&mut self, inner: String) -> Result<(), ClientError> {
        tracing::debug!(inner = ?&inner);
        let inner_dbg = inner.clone();
        match serde_json::from_str::<Incoming>(&inner) {
            Err(e) => {
                tracing::error!(e = ?&e);
                tracing::error!("inner: {}", inner_dbg);
            }
            Ok(Incoming::HelloMsg(_)) => {}
            Ok(Incoming::Response(resp)) => {
                if resp.raw.is_none() {
                    if let Entry::Occupied(stream) = self.subscriptions.entry(1u64) {
                        if let Err(err) = stream.get().unbounded_send(resp) {
                            if err.is_disconnected() {
                                // subscription channel was closed on the receiver end
                                stream.remove();
                            }
                            return Err(to_client_error(err));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle(&mut self, resp: Message) -> Result<(), ClientError> {
        match resp {
            Message::Text(inner) => self.handle_text(inner).await,
            Message::Ping(inner) => self.handle_ping(inner).await,
            Message::Pong(_) => Ok(()), // Server is allowed to send unsolicited pongs.
            Message::Close(Some(frame)) => Err(ClientError::WsClosed(frame)),
            Message::Close(None) => Err(ClientError::UnexpectedClose),
            Message::Binary(buf) => Err(ClientError::UnexpectedBinary(buf)),
        }
    }

    /// Processes 1 instruction or 1 incoming websocket message
    #[allow(clippy::single_match)]
    async fn tick(&mut self) -> Result<(), ClientError> {
        futures_util::select! {
            // Handle requests
            instruction = self.instructions.select_next_some() => {
                self.service(instruction).await?;
            },
            // Handle ws messages
            resp = self.ws.next() => match resp {
                Some(Ok(resp)) => self.handle(resp).await?,
                // TODO: Log the error?
                Some(Err(_)) => {},
                None => {
                    return Err(ClientError::UnexpectedClose);
                },
            }
        };

        Ok(())
    }
}

// TrySendError is private :(
fn to_client_error<T: Debug>(err: T) -> ClientError {
    ClientError::ChannelError(format!("{:?}", err))
}

#[derive(Error, Debug)]
/// Error thrown when sending a WS message
pub enum ClientError {
    /// Thrown if deserialization failed
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error(transparent)]
    /// Thrown if the response could not be parsed
    JsonRpcError(#[from] JsonRpcError),

    /// Thrown if the websocket responds with binary data
    #[error("Websocket responded with unexpected binary data")]
    UnexpectedBinary(Vec<u8>),

    /// Thrown if there's an error over the WS connection
    #[error(transparent)]
    TungsteniteError(#[from] WsError),

    #[error("{0}")]
    ChannelError(String),

    #[error(transparent)]
    Canceled(#[from] oneshot::Canceled),

    /// Remote server sent a Close message
    #[error("Websocket closed with info: {0:?}")]
    WsClosed(CloseFrame<'static>),

    /// Something caused the websocket to close
    #[error("WebSocket connection closed unexpectedly")]
    UnexpectedClose,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{Network, System},
        ws::models::{WatchConfig, WatchRequest},
    };
    use std::collections::HashMap;
    use tokio::fs::read_to_string;

    #[tokio::test]
    async fn request() {
        let bc = Blockchain {
            system: System::Ethereum,
            network: Network::Polygon,
        };
        let ws = Ws::connect("wss://api.blocknative.com/v0", "", bc)
            .await
            .unwrap();

        let s = read_to_string("examples/quickswap.json").await.unwrap();
        let abi = serde_json::from_str(&s).unwrap();

        let mut filters = HashMap::new();
        filters.insert(
            "contractCall.params.path".to_string(),
            "0xC250e9987A032ACAC293d838726C511E6E1C029d".to_string(),
        );

        let config=  WatchConfig {
            scope: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".to_string(),
            filters: vec![filters],
            abi,
            watch_address: true,
        };

       let mut stream = ws.listen(config).await.unwrap();

        while let Some(event) = stream.next().await {
            println!("got event: {:?}", event);
            let txn = event.event.unwrap().transaction.unwrap();
            // let ether_tx: ethers::prelude::Transaction = txn.into();

            // println("")
            break;
        }
    }
}
