use std::{sync::atomic::{AtomicBool, Ordering}, net::SocketAddrV4, io};

use log::info;
use tokio::{sync::watch::Receiver, net::{TcpListener, TcpStream}};

use crate::http::host_info::HostInfo;

pub mod host_info;

const HTTP_RESPONSE_BASE: &'static str = "HTTP/1.1 200\r\nContent-Type: application/json\r\n";

pub struct OQHTTPHandler <'hostinfo>{
        thread_rx: Receiver<AtomicBool>,
        bound_addr: Option<SocketAddrV4>,
        host_info: HostInfo<'hostinfo>,
}

impl <'hostinfo> OQHTTPHandler<'hostinfo> {
    
    pub fn new(bind_addr: SocketAddrV4, host_info: HostInfo<'hostinfo>, thread_rx: Receiver<AtomicBool>) -> Self {
        OQHTTPHandler { thread_rx, bound_addr: Some(bind_addr), host_info }
    }

    pub async fn handle_connection(&self, tcp_stream: &TcpStream) {

        let mut full_buffer = Vec::new();
        let mut buffer: [u8; 1024] = [0x0; 1024];
        tcp_stream.readable().await.unwrap();

        loop {

            match tcp_stream.try_read(&mut buffer) {
                Ok(0) => break,
                Ok(_b) => {

                    full_buffer.extend_from_slice(&buffer);
                    if _b < 1024 {break};
                    buffer.fill_with(||0x0);
                },
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(_e) => break,
            }
        }

        //println!("-=== Buffer ===-\n{}", String::from_utf8_lossy(&full_buffer).to_owned());

        let h_info = serde_json::to_string(&self.host_info).unwrap();
        let http_res = format!("{}Content-Length: {}\r\n\r\n{}", HTTP_RESPONSE_BASE, h_info.len(), h_info);

        let bytes_sent = tcp_stream.try_write(http_res.as_bytes()).unwrap();
        info!("Sent {} bytes", bytes_sent);
    }
}

pub async fn start(http_handler: OQHTTPHandler<'_>) {
        
    let tcp_listener = TcpListener::bind(http_handler.bound_addr.unwrap()).await.unwrap();

    let mut state = true;

    while state {

        if let Ok((s, a)) = tcp_listener.accept().await {
            info!("Got connection from: {}", a);
            http_handler.handle_connection(&s).await;
        }

        if http_handler.thread_rx.has_changed().unwrap() {
            state = http_handler.thread_rx.borrow().load(Ordering::SeqCst);
        }
    }
}