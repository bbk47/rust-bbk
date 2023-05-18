use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct BbkCliOption {
    pub mode: String,
    pub listen_addr: String,
    pub listen_port: i64,
    pub listen_http_port: i64,
    #[serde(default = "loglevel_default")]
    pub log_level: String,
    pub tunnel_opts: Option<TunnelOpts>,
    #[serde(default = "ping_default")]
    pub ping: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct BbkSerOption {
    pub mode: String,
    pub listen_addr: String,
    pub listen_port: i64,
    #[serde(default = "loglevel_default")]
    pub log_level: String,
    pub method: String,
    pub password: String,
    pub work_mode: String,
    pub work_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelOpts {
    pub protocol: String,
    pub secure: bool,
    pub host: String,
    pub port: String,
    pub path: String,
    pub method: String,
    pub password: String,
}

fn loglevel_default() -> String {
    String::from("info")
}

fn ping_default() -> bool {
    false
}
