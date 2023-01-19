// Copyright (c) 2022 MASSA LABS <info@massa.net>
//! Utilities for a massa client

#![warn(missing_docs)]
#![warn(unused_crate_dependencies)]

use http::header::HeaderName;
use jsonrpsee::core::client::{
    CertificateStore, ClientT, IdKind, Subscription, SubscriptionClientT,
};
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::rpc_params;
use jsonrpsee::types::error::CallError;
use jsonrpsee::types::ErrorObject;
use jsonrpsee::ws_client::{HeaderMap, HeaderValue, WsClient, WsClientBuilder};
use massa_api_exports::{
    address::AddressInfo,
    block::{BlockInfo, BlockSummary},
    datastore::{DatastoreEntryInput, DatastoreEntryOutput},
    endorsement::EndorsementInfo,
    execution::{ExecuteReadOnlyResponse, ReadOnlyBytecodeExecution, ReadOnlyCall},
    node::NodeStatus,
    operation::{OperationInfo, OperationInput},
    TimeInterval,
};
use massa_models::{
    address::Address,
    block::FilledBlock,
    block_header::BlockHeader,
    block_id::BlockId,
    clique::Clique,
    composite::PubkeySig,
    endorsement::EndorsementId,
    execution::EventFilter,
    node::NodeId,
    operation::{Operation, OperationId},
    output_event::SCOutputEvent,
    prehash::{PreHashMap, PreHashSet},
    version::Version,
};

use jsonrpsee::{core::Error as JsonRpseeError, core::RpcResult, http_client::HttpClientBuilder};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

mod config;
pub use config::ClientConfig;
pub use config::HttpConfig;
pub use config::WsConfig;

/// Client
pub struct Client {
    /// public component
    pub public: RpcClient,
    /// private component
    pub private: RpcClient,
}

impl Client {
    /// creates a new client
    pub async fn new(
        ip: IpAddr,
        public_port: u16,
        private_port: u16,
        http_config: &HttpConfig,
    ) -> Client {
        let public_socket_addr = SocketAddr::new(ip, public_port);
        let private_socket_addr = SocketAddr::new(ip, private_port);
        let public_url = format!("http://{}", public_socket_addr);
        let private_url = format!("http://{}", private_socket_addr);
        Client {
            public: RpcClient::from_url(&public_url, http_config).await,
            private: RpcClient::from_url(&private_url, http_config).await,
        }
    }
}

/// Rpc client
pub struct RpcClient {
    http_client: HttpClient,
}

impl RpcClient {
    /// Default constructor
    pub async fn from_url(url: &str, http_config: &HttpConfig) -> RpcClient {
        RpcClient {
            http_client: http_client_from_url(url, http_config).await,
        }
    }

    /// Gracefully stop the node.
    pub async fn stop_node(&self) -> RpcResult<()> {
        self.http_client.request("stop_node", rpc_params![]).await
    }

    /// Sign message with node's key.
    /// Returns the public key that signed the message and the signature.
    pub async fn node_sign_message(&self, message: Vec<u8>) -> RpcResult<PubkeySig> {
        self.http_client
            .request("node_sign_message", rpc_params![message])
            .await
    }

    /// Add a vector of new secret keys for the node to use to stake.
    /// No confirmation to expect.
    pub async fn add_staking_secret_keys(&self, secret_keys: Vec<String>) -> RpcResult<()> {
        self.http_client
            .request("add_staking_secret_keys", rpc_params![secret_keys])
            .await
    }

    /// Remove a vector of addresses used to stake.
    /// No confirmation to expect.
    pub async fn remove_staking_addresses(&self, addresses: Vec<Address>) -> RpcResult<()> {
        self.http_client
            .request("remove_staking_addresses", rpc_params![addresses])
            .await
    }

    /// Return hash-set of staking addresses.
    pub async fn get_staking_addresses(&self) -> RpcResult<PreHashSet<Address>> {
        self.http_client
            .request("get_staking_addresses", rpc_params![])
            .await
    }

    /// Bans given ip address(es)
    /// No confirmation to expect.
    pub async fn node_ban_by_ip(&self, ips: Vec<IpAddr>) -> RpcResult<()> {
        self.http_client
            .request("node_ban_by_ip", rpc_params![ips])
            .await
    }

    /// Bans given node id(s)
    /// No confirmation to expect.
    pub async fn node_ban_by_id(&self, ids: Vec<NodeId>) -> RpcResult<()> {
        self.http_client
            .request("node_ban_by_id", rpc_params![ids])
            .await
    }

    /// Unban given ip address(es)
    /// No confirmation to expect.
    pub async fn node_unban_by_ip(&self, ips: Vec<IpAddr>) -> RpcResult<()> {
        self.http_client
            .request("node_unban_by_ip", rpc_params![ips])
            .await
    }

    /// Unban given node id(s)
    /// No confirmation to expect.
    pub async fn node_unban_by_id(&self, ids: Vec<NodeId>) -> RpcResult<()> {
        self.http_client
            .request("node_unban_by_id", rpc_params![ids])
            .await
    }

    /// Returns node peers whitelist IP address(es).
    pub async fn node_peers_whitelist(&self) -> RpcResult<Vec<IpAddr>> {
        self.http_client
            .request("node_peers_whitelist", rpc_params![])
            .await
    }

    /// Add IP address(es) to node peers whitelist.
    pub async fn node_add_to_peers_whitelist(&self, ips: Vec<IpAddr>) -> RpcResult<()> {
        self.http_client
            .request("node_add_to_peers_whitelist", rpc_params![ips])
            .await
    }

    /// Remove IP address(es) to node peers whitelist.
    pub async fn node_remove_from_peers_whitelist(&self, ips: Vec<IpAddr>) -> RpcResult<()> {
        self.http_client
            .request("node_remove_from_peers_whitelist", rpc_params![ips])
            .await
    }

    /// Returns node bootsrap whitelist IP address(es).
    pub async fn node_bootstrap_whitelist(&self) -> RpcResult<Vec<IpAddr>> {
        self.http_client
            .request("node_bootstrap_whitelist", rpc_params![])
            .await
    }

    /// Allow everyone to bootsrap from the node.
    /// remove bootsrap whitelist configuration file.
    pub async fn node_bootstrap_whitelist_allow_all(&self) -> RpcResult<()> {
        self.http_client
            .request("node_bootstrap_whitelist_allow_all", rpc_params![])
            .await
    }

    /// Add IP address(es) to node bootsrap whitelist.
    pub async fn node_add_to_bootstrap_whitelist(&self, ips: Vec<IpAddr>) -> RpcResult<()> {
        self.http_client
            .request("node_add_to_bootstrap_whitelist", rpc_params![ips])
            .await
    }

    /// Remove IP address(es) to bootsrap whitelist.
    pub async fn node_remove_from_bootstrap_whitelist(&self, ips: Vec<IpAddr>) -> RpcResult<()> {
        self.http_client
            .request("node_remove_from_bootstrap_whitelist", rpc_params![ips])
            .await
    }

    /// Returns node bootsrap blacklist IP address(es).
    pub async fn node_bootstrap_blacklist(&self) -> RpcResult<Vec<IpAddr>> {
        self.http_client
            .request("node_bootstrap_blacklist", rpc_params![])
            .await
    }

    /// Add IP address(es) to node bootsrap blacklist.
    pub async fn node_add_to_bootstrap_blacklist(&self, ips: Vec<IpAddr>) -> RpcResult<()> {
        self.http_client
            .request("node_add_to_bootstrap_blacklist", rpc_params![ips])
            .await
    }

    /// Remove IP address(es) to bootsrap blacklist.
    pub async fn node_remove_from_bootstrap_blacklist(&self, ips: Vec<IpAddr>) -> RpcResult<()> {
        self.http_client
            .request("node_remove_from_bootstrap_blacklist", rpc_params![ips])
            .await
    }

    ////////////////
    // public-api //
    ////////////////

    // Explorer (aggregated stats)

    /// summary of the current state: time, last final blocks (hash, thread, slot, timestamp), clique count, connected nodes count
    pub async fn get_status(&self) -> RpcResult<NodeStatus> {
        self.http_client.request("get_status", rpc_params![]).await
    }

    pub(crate) async fn _get_cliques(&self) -> RpcResult<Vec<Clique>> {
        self.http_client.request("get_cliques", rpc_params![]).await
    }

    // Debug (specific information)

    /// Returns the active stakers and their roll counts for the current cycle.
    pub(crate) async fn _get_stakers(&self) -> RpcResult<PreHashMap<Address, u64>> {
        self.http_client.request("get_stakers", rpc_params![]).await
    }

    /// Returns operation(s) information associated to a given list of operation(s) ID(s).
    pub async fn get_operations(
        &self,
        operation_ids: Vec<OperationId>,
    ) -> RpcResult<Vec<OperationInfo>> {
        self.http_client
            .request("get_operations", rpc_params![operation_ids])
            .await
    }

    /// Returns endorsement(s) information associated to a given list of endorsement(s) ID(s)
    pub async fn get_endorsements(
        &self,
        endorsement_ids: Vec<EndorsementId>,
    ) -> RpcResult<Vec<EndorsementInfo>> {
        self.http_client
            .request("get_endorsements", rpc_params![endorsement_ids])
            .await
    }

    /// Returns block(s) information associated to a given list of block(s) ID(s)
    pub async fn get_blocks(&self, block_ids: Vec<BlockId>) -> RpcResult<BlockInfo> {
        self.http_client
            .request("get_blocks", rpc_params![block_ids])
            .await
    }

    /// Get events emitted by smart contracts with various filters
    pub async fn get_filtered_sc_output_event(
        &self,
        filter: EventFilter,
    ) -> RpcResult<Vec<SCOutputEvent>> {
        self.http_client
            .request("get_filtered_sc_output_event", rpc_params![filter])
            .await
    }

    /// Get the block graph within the specified time interval.
    /// Optional parameters: from `<time_start>` (included) and to `<time_end>` (excluded) millisecond timestamp
    pub(crate) async fn _get_graph_interval(
        &self,
        time_interval: TimeInterval,
    ) -> RpcResult<Vec<BlockSummary>> {
        self.http_client
            .request("get_graph_interval", rpc_params![time_interval])
            .await
    }

    /// Get info by addresses
    pub async fn get_addresses(&self, addresses: Vec<Address>) -> RpcResult<Vec<AddressInfo>> {
        self.http_client
            .request("get_addresses", rpc_params![addresses])
            .await
    }

    /// Get datastore entries
    pub async fn get_datastore_entries(
        &self,
        input: Vec<DatastoreEntryInput>,
    ) -> RpcResult<Vec<DatastoreEntryOutput>> {
        self.http_client
            .request("get_datastore_entries", rpc_params![input])
            .await
    }

    // User (interaction with the node)

    /// Adds operations to pool. Returns operations that were ok and sent to pool.
    pub async fn send_operations(
        &self,
        operations: Vec<OperationInput>,
    ) -> RpcResult<Vec<OperationId>> {
        self.http_client
            .request("send_operations", rpc_params![operations])
            .await
    }

    /// execute read only bytecode
    pub async fn execute_read_only_bytecode(
        &self,
        read_only_execution: ReadOnlyBytecodeExecution,
    ) -> RpcResult<ExecuteReadOnlyResponse> {
        self.http_client
            .request::<Vec<ExecuteReadOnlyResponse>, Vec<Vec<ReadOnlyBytecodeExecution>>>(
                "execute_read_only_bytecode",
                vec![vec![read_only_execution]],
            )
            .await?
            .pop()
            .ok_or_else(|| {
                JsonRpseeError::Custom("missing return value on execute_read_only_bytecode".into())
            })
    }

    /// execute read only SC call
    pub async fn execute_read_only_call(
        &self,
        read_only_execution: ReadOnlyCall,
    ) -> RpcResult<ExecuteReadOnlyResponse> {
        self.http_client
            .request::<Vec<ExecuteReadOnlyResponse>, Vec<Vec<ReadOnlyCall>>>(
                "execute_read_only_call",
                vec![vec![read_only_execution]],
            )
            .await?
            .pop()
            .ok_or_else(|| {
                JsonRpseeError::Custom("missing return value on execute_read_only_call".into())
            })
    }
}

/// Client V2
pub struct ClientV2 {
    /// API V2 component
    pub api: RpcClientV2,
}

impl ClientV2 {
    /// creates a new client
    pub async fn new(
        ip: IpAddr,
        api_port: u16,
        http_config: &HttpConfig,
        ws_config: &WsConfig,
    ) -> ClientV2 {
        let api_socket_addr = SocketAddr::new(ip, api_port);
        ClientV2 {
            api: RpcClientV2::from_url(api_socket_addr, http_config, ws_config).await,
        }
    }
}

/// Rpc V2 client
pub struct RpcClientV2 {
    http_client: Option<HttpClient>,
    ws_client: Option<WsClient>,
}

impl RpcClientV2 {
    /// Default constructor
    pub async fn from_url(
        socket_addr: SocketAddr,
        http_config: &HttpConfig,
        ws_config: &WsConfig,
    ) -> RpcClientV2 {
        let http_url = format!("http://{}", socket_addr);
        let ws_url = format!("ws://{}", socket_addr);

        if http_config.enabled && !ws_config.enabled {
            let http_client = http_client_from_url(&http_url, http_config).await;
            return RpcClientV2 {
                http_client: Some(http_client),
                ws_client: None,
            };
        } else if !http_config.enabled && ws_config.enabled {
            let ws_client = ws_client_from_url(&ws_url, ws_config).await;
            return RpcClientV2 {
                http_client: None,
                ws_client: Some(ws_client),
            };
        } else if !http_config.enabled && !ws_config.enabled {
            panic!("wrong client configuration, you can't disable both http and ws");
        }

        let http_client = http_client_from_url(&http_url, http_config).await;
        let ws_client = ws_client_from_url(&ws_url, ws_config).await;

        RpcClientV2 {
            http_client: Some(http_client),
            ws_client: Some(ws_client),
        }
    }

    ////////////////
    //   API V2   //
    ////////////////
    //
    // Experimental APIs. They might disappear, and they will change //

    /// Get Massa node version
    pub async fn get_version(&self) -> RpcResult<Version> {
        if let Some(client) = self.http_client.as_ref() {
            client.request("get_version", rpc_params![]).await
        } else {
            Err(JsonRpseeError::Custom(
                "error, no Http client instance found".to_owned(),
            ))
        }
    }

    /// New produced blocks
    pub async fn subscribe_new_blocks(
        &self,
    ) -> Result<Subscription<BlockInfo>, jsonrpsee::core::Error> {
        if let Some(client) = self.ws_client.as_ref() {
            client
                .subscribe(
                    "subscribe_new_blocks",
                    rpc_params![],
                    "unsubscribe_new_blocks",
                )
                .await
        } else {
            Err(CallError::Custom(ErrorObject::owned(
                -32080,
                "error, no WebSocket client instance found".to_owned(),
                None::<()>,
            ))
            .into())
        }
    }

    /// New produced blocks headers
    pub async fn subscribe_new_blocks_headers(
        &self,
    ) -> Result<Subscription<BlockHeader>, jsonrpsee::core::Error> {
        if let Some(client) = self.ws_client.as_ref() {
            client
                .subscribe(
                    "subscribe_new_blocks_headers",
                    rpc_params![],
                    "unsubscribe_new_blocks_headers",
                )
                .await
        } else {
            Err(CallError::Custom(ErrorObject::owned(
                -32080,
                "error, no WebSocket client instance found".to_owned(),
                None::<()>,
            ))
            .into())
        }
    }

    /// New produced blocks with operations content.
    pub async fn subscribe_new_filled_blocks(
        &self,
    ) -> Result<Subscription<FilledBlock>, jsonrpsee::core::Error> {
        if let Some(client) = self.ws_client.as_ref() {
            client
                .subscribe(
                    "subscribe_new_filled_blocks",
                    rpc_params![],
                    "unsubscribe_new_filled_blocks",
                )
                .await
        } else {
            Err(CallError::Custom(ErrorObject::owned(
                -32080,
                "error, no WebSocket client instance found".to_owned(),
                None::<()>,
            ))
            .into())
        }
    }

    /// New produced operations.
    pub async fn subscribe_new_operations(
        &self,
    ) -> Result<Subscription<Operation>, jsonrpsee::core::Error> {
        if let Some(client) = self.ws_client.as_ref() {
            client
                .subscribe(
                    "subscribe_new_operations",
                    rpc_params![],
                    "unsubscribe_new_operations",
                )
                .await
        } else {
            Err(CallError::Custom(ErrorObject::owned(
                -32080,
                "error, no WebSocket client instance found".to_owned(),
                None::<()>,
            ))
            .into())
        }
    }
}

async fn http_client_from_url(url: &str, http_config: &HttpConfig) -> HttpClient {
    match HttpClientBuilder::default()
        .max_request_body_size(http_config.client_config.max_request_body_size)
        .request_timeout(http_config.client_config.request_timeout.to_duration())
        .max_concurrent_requests(http_config.client_config.max_concurrent_requests)
        .certificate_store(get_certificate_store(
            http_config.client_config.certificate_store.as_str(),
        ))
        .id_format(get_id_kind(http_config.client_config.id_kind.as_str()))
        .set_headers(get_headers(&http_config.client_config.headers))
        .build(url)
    {
        Ok(http_client) => http_client,
        Err(_) => panic!("unable to create Http client."),
    }
}

async fn ws_client_from_url(url: &str, ws_config: &WsConfig) -> WsClient
where
    WsClient: SubscriptionClientT,
{
    match WsClientBuilder::default()
        .max_request_body_size(ws_config.client_config.max_request_body_size)
        .request_timeout(ws_config.client_config.request_timeout.to_duration())
        .max_concurrent_requests(ws_config.client_config.max_concurrent_requests)
        .certificate_store(get_certificate_store(
            ws_config.client_config.certificate_store.as_str(),
        ))
        .id_format(get_id_kind(ws_config.client_config.id_kind.as_str()))
        .set_headers(get_headers(&ws_config.client_config.headers))
        .max_notifs_per_subscription(ws_config.max_notifs_per_subscription)
        .max_redirections(ws_config.max_redirections)
        .build(url)
        .await
    {
        Ok(ws_client) => ws_client,
        Err(_) => panic!("unable to create WebSocket client"),
    }
}

fn get_certificate_store(certificate_store: &str) -> CertificateStore {
    match certificate_store {
        "Native" => CertificateStore::Native,
        "WebPki" => CertificateStore::WebPki,
        _ => CertificateStore::Native,
    }
}

fn get_id_kind(id_kind: &str) -> IdKind {
    match id_kind {
        "Number" => IdKind::Number,
        "String" => IdKind::String,
        _ => IdKind::Number,
    }
}

fn get_headers(headers: &[(String, String)]) -> HeaderMap {
    let mut headers_map = HeaderMap::new();
    headers.iter().for_each(|(key, value)| {
        let header_name = match HeaderName::from_str(key.as_str()) {
            Ok(header_name) => header_name,
            Err(_) => panic!("invalid header name: {:?}", key),
        };
        let header_value = match HeaderValue::from_str(value.as_str()) {
            Ok(header_name) => header_name,
            Err(_) => panic!("invalid header value: {:?}", value),
        };
        headers_map.insert(header_name, header_value);
    });

    headers_map
}
