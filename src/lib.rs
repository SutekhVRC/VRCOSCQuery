//! OSCQuery for VRChat apps.
//!
//! - [`OscQueryService`] advertises your app's OSC port over mDNS and serves
//!   the OSCQuery HTTP endpoints, so VRChat finds your app and sends it OSC
//!   on a port you chose, alongside its usual 9001. No fixed port to clash
//!   with other apps.
//! - [`discovery`] finds OSCQuery services, such as VRChat's own, with a
//!   timeout, and reads their `HOST_INFO` (e.g. the port VRChat receives
//!   OSC on) and node tree (every avatar parameter and its current value).
//!
//! Everything runs on the caller's tokio runtime.
//!
//! ```no_run
//! # async fn example() -> Result<(), vrcoscquery::OQError> {
//! use std::net::SocketAddr;
//! use std::time::Duration;
//!
//! // Your OSC socket, bound on a free port
//! // 0.0.0.0: VRChat sends to the LAN address, whatever HOST_INFO says
//! let osc = std::net::UdpSocket::bind("0.0.0.0:0")?;
//! let SocketAddr::V4(osc_addr) = osc.local_addr()? else { unreachable!() };
//!
//! // VRChat will send avatar parameters to `osc_addr`
//! let service = vrcoscquery::OscQueryService::start("MyApp", osc_addr).await?;
//!
//! // Where VRChat receives OSC
//! if let Some(vrchat) = vrcoscquery::discovery::find_vrchat(Duration::from_secs(3)).await? {
//!     let info = vrcoscquery::discovery::host_info(&vrchat, Duration::from_secs(3)).await?;
//!     println!("VRChat listens on {}:{}", info.osc_ip, info.osc_port);
//! }
//!
//! service.stop();
//! # Ok(())
//! # }
//! ```

pub mod discovery;
pub mod http;
pub mod mdns;
pub mod node;
mod service;

#[cfg(test)]
mod tests;

pub use discovery::Endpoint;
pub use service::OscQueryService;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OQError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("mDNS error: {0}")]
    Mdns(#[from] mdns_sd::Error),
    #[error("the background task stopped unexpectedly")]
    TaskFailed,
}
