// Copyright (c) 2023 MASSA LABS <info@massa.net>

use massa_time::MassaTime;

/// Client common settings.
/// the client common settings
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// maximum size in bytes of a request.
    pub max_request_body_size: u32,
    /// maximum size in bytes of a response.
    pub request_timeout: MassaTime,
    /// maximum concurrent requests.
    pub max_concurrent_requests: usize,
    /// certificate_store, `Native` or `WebPki`
    pub certificate_store: String,
    /// JSON-RPC request object id data type, `Number` or `String`
    pub id_kind: String,
    /// max length for logging for requests and responses. Logs bigger than this limit will be truncated.
    pub max_log_length: u32,
    /// custom headers to pass with every request.
    pub headers: Vec<(String, String)>,
}

/// Http client settings.
/// the Http client settings
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// common client configuration.
    pub client_config: ClientConfig,
    /// whether to enable HTTP.
    pub enabled: bool,
}

/// WebSocket client settings.
/// the WebSocket client settings
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// common client configuration.
    pub client_config: ClientConfig,
    /// whether to enable WS.
    pub enabled: bool,
    /// Max notifications per subscription.
    pub max_notifs_per_subscription: usize,
    /// Max number of redirections.
    pub max_redirections: usize,
}
