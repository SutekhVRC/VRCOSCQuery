use super::info;
use crate::http::json_models::HostInfo;
use std::{
    io,
    net::SocketAddrV4,
    sync::atomic::{AtomicBool, Ordering},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch::Receiver,
};

pub mod json_models;
pub mod node;

const HTTP_RESPONSE_BASE: &'static str = "HTTP/1.1 200\r\nContent-Type: application/json\r\n";

const HTTP_INDEX: &'static str = "{\"DESCRIPTION\": \"root node\",\"FULL_PATH\": \"/\",\"ACCESS\": 0,\"CONTENTS\": {\"avatar\": {\"FULL_PATH\": \"/avatar\",\"ACCESS\": 0,\"CONTENTS\": {\"change\": {\"DESCRIPTION\": \"Avatar ID, updated whenever the user switches into a valid avatar.\",\"FULL_PATH\": \"/avatar/change\",\"ACCESS\": 1,\"TYPE\": \"s\",\"VALUE\": [\"avtr_id_lol\"]}}}}}";

pub struct OQHTTPHandler<'hostinfo> {
    thread_rx: Receiver<AtomicBool>,
    bound_addr: Option<SocketAddrV4>,
    host_info: HostInfo<'hostinfo>,
}

impl<'hostinfo> OQHTTPHandler<'hostinfo> {
    pub fn new(
        bind_addr: SocketAddrV4,
        host_info: HostInfo<'hostinfo>,
        thread_rx: Receiver<AtomicBool>,
    ) -> Self {
        OQHTTPHandler {
            thread_rx,
            bound_addr: Some(bind_addr),
            host_info,
        }
    }

    async fn http_route(&self, buffer: Vec<u8>, tcp_stream: &TcpStream) {
        if buffer.starts_with("GET / HTTP/".as_bytes()) {
            let http_index = String::from_utf8_lossy(HTTP_INDEX.as_bytes());
            let http_res = format!(
                "{}Content-Length: {}\r\n\r\n{}",
                HTTP_RESPONSE_BASE,
                HTTP_INDEX.len(),
                http_index
            );
            tcp_stream.try_write(http_res.as_bytes()).unwrap();
        } else if buffer.starts_with("GET /?HOST_INFO HTTP/".as_bytes()) {
            let h_info = serde_json::to_string(&self.host_info).unwrap();
            let http_res = format!(
                "{}Content-Length: {}\r\n\r\n{}",
                HTTP_RESPONSE_BASE,
                h_info.len(),
                h_info
            );

            let bytes_sent = tcp_stream.try_write(http_res.as_bytes()).unwrap();
            info!("[+] Sent {} bytes", bytes_sent);
        } else { //LOLOLOL
        }
    }

    pub async fn handle_connection(&self, tcp_stream: &TcpStream) {
        let mut full_buffer = Vec::new();
        let mut buffer: [u8; 1024] = [0x0; 1024];
        tcp_stream.readable().await.unwrap();

        loop {
            match tcp_stream.try_read(&mut buffer) {
                Ok(0) => break,
                Ok(b) => {
                    full_buffer.extend_from_slice(&buffer);
                    if b < 1024 {
                        break;
                    };
                    buffer.fill_with(|| 0x0);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(_e) => break,
            }
        }

        self.http_route(full_buffer, tcp_stream).await;
    }
}

pub async fn start(http_handler: OQHTTPHandler<'_>) {
    info!("[*] HTTP Start()");
    let tcp_listener = TcpListener::bind(http_handler.bound_addr.unwrap())
        .await
        .unwrap();
    info!("[+] Bound HTTP service.");

    let mut state = true;

    while state {
        if let Ok((s, a)) = tcp_listener.accept().await {
            info!("[+] Got connection from: {}", a);
            http_handler.handle_connection(&s).await;
        }

        if http_handler.thread_rx.has_changed().unwrap() {
            state = http_handler.thread_rx.borrow().load(Ordering::SeqCst);
        }
    }
}
