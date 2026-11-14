//! The OSCQuery HTTP server an [`OscQueryService`](crate::OscQueryService)
//! runs, and the JSON models it serves.
//!
//! Routes:
//! - `/?HOST_INFO`: the service's name and OSC address ([`HostInfo`]).
//! - `/` and any path in the node tree (e.g. `/avatar`): that node.
//! - Anything else: 404.
//!
//! Each connection is answered on its own task and closed after one
//! response, so a slow or stuck client can't hold up others.

pub mod json_models;

pub use json_models::{HostInfo, HostInfoExtensions};

use std::sync::Arc;
use std::time::Duration;

use log::{debug, trace};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

/// Longest a client may take to send its request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Requests are small; anything longer is refused.
const MAX_REQUEST: usize = 8 * 1024;

/// What the server answers with, prepared once.
pub(crate) struct Responses {
    host_info: String,
    tree: Value,
}

impl Responses {
    pub(crate) fn new(host_info: &HostInfo, tree: Value) -> Result<Self, serde_json::Error> {
        Ok(Self {
            host_info: serde_json::to_string(host_info)?,
            tree,
        })
    }

    /// The status and JSON body for a request target such as `/?HOST_INFO`
    /// or `/avatar`.
    fn route(&self, target: &str) -> (u16, String) {
        if target == "/?HOST_INFO" {
            return (200, self.host_info.clone());
        }
        // Other queries (e.g. ?VALUE) ask for one attribute; the whole node
        // includes it
        let path = target.split('?').next().unwrap_or("/");
        match find_node(&self.tree, path) {
            Some(node) => (200, node.to_string()),
            None => (404, r#"{"error":"not found"}"#.to_owned()),
        }
    }
}

/// The node in `tree` whose FULL_PATH is `path`.
fn find_node<'a>(tree: &'a Value, path: &str) -> Option<&'a Value> {
    let path = if path.len() > 1 {
        path.trim_end_matches('/')
    } else {
        path
    };
    if tree.get("FULL_PATH").and_then(Value::as_str) == Some(path) {
        return Some(tree);
    }
    tree.get("CONTENTS")?
        .as_object()?
        .values()
        .find_map(|child| find_node(child, path))
}

/// Answers requests until `shutdown` changes or its sender is dropped.
pub(crate) async fn serve(
    listener: TcpListener,
    responses: Arc<Responses>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            _ = shutdown.changed() => break,
            accepted = listener.accept() => match accepted {
                Ok((stream, peer)) => {
                    trace!("OSCQuery HTTP connection from {}", peer);
                    let responses = responses.clone();
                    tokio::spawn(async move {
                        if let Err(e) = answer(stream, &responses).await {
                            debug!("OSCQuery HTTP request from {} failed: {}", peer, e);
                        }
                    });
                }
                Err(e) => debug!("OSCQuery HTTP accept failed: {}", e),
            },
        }
    }
    debug!("OSCQuery HTTP server stopped");
}

async fn answer(mut stream: TcpStream, responses: &Responses) -> std::io::Result<()> {
    let request = tokio::time::timeout(REQUEST_TIMEOUT, read_request(&mut stream))
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "request timed out"))??;

    let (status, body) = match request_target(&request) {
        Some(target) => responses.route(target),
        None => (400, r#"{"error":"bad request"}"#.to_owned()),
    };
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Bad Request",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await
}

/// Reads until the end of the request headers.
async fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];
    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if request.windows(4).any(|w| w == b"\r\n\r\n") || request.len() >= MAX_REQUEST {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&request).into_owned())
}

/// The target of a GET request line (`GET /?HOST_INFO HTTP/1.1`).
fn request_target(request: &str) -> Option<&str> {
    let mut parts = request.lines().next()?.split_whitespace();
    if parts.next()? != "GET" {
        return None;
    }
    let target = parts.next()?;
    parts.next()?.starts_with("HTTP/").then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn responses() -> Responses {
        let info = HostInfo {
            name: "Test".into(),
            extensions: HostInfoExtensions::default(),
            osc_ip: "127.0.0.1".into(),
            osc_port: 9123,
            osc_transport: "UDP".into(),
        };
        Responses::new(&info, crate::service::app_tree()).unwrap()
    }

    #[test]
    fn routes_host_info_nodes_and_unknown_paths() {
        let r = responses();
        let (status, body) = r.route("/?HOST_INFO");
        assert_eq!(status, 200);
        assert!(body.contains("9123"));

        for (target, path) in [
            ("/", "/"),
            ("/avatar", "/avatar"),
            ("/avatar/", "/avatar"),
            ("/avatar/change?VALUE", "/avatar/change"),
        ] {
            let (status, body) = r.route(target);
            assert_eq!(status, 200, "{target}");
            let node: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(node["FULL_PATH"], path, "{target}");
        }

        assert_eq!(r.route("/nope").0, 404);
    }

    #[test]
    fn reads_the_target_of_get_requests_only() {
        assert_eq!(
            request_target("GET /?HOST_INFO HTTP/1.1\r\nHost: x\r\n\r\n"),
            Some("/?HOST_INFO")
        );
        assert_eq!(request_target("POST / HTTP/1.1\r\n\r\n"), None);
        assert_eq!(request_target("GET /\r\n"), None);
        assert_eq!(request_target(""), None);
    }
}
