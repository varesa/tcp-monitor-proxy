use tokio::net::{TcpListener, TcpStream};
use crate::errors::MirrorError;
use tokio_util::codec::{Framed, BytesCodec};
use futures::{StreamExt, SinkExt};
use bytes::Bytes;
use futures::channel::mpsc::{channel, Sender};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::stream::SplitSink;

struct Shared {
    clients: Vec<SplitSink<Framed<TcpStream, BytesCodec>, Bytes>>,
}

impl Shared {
    fn new() -> Self {
        Shared {
            clients: Vec::new(),
        }
    }
}

pub struct Monitor {
    port: u16,
    tx: Option<Sender<Bytes>>,
    shared: Arc<Mutex<Shared>>,
}

impl Monitor {
    pub async fn try_new(port: u16) -> Result<Self, MirrorError> {
        let (tx, mut rx) = channel(65565);

        let monitor = Monitor {
            port,
            tx: Some(tx),
            shared: Arc::new(Mutex::new(Shared::new())),
        };

        let shared_forwarder = monitor.shared.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = rx.next().await {
                    let mut state = shared_forwarder.lock().await;
                    let mut client_errors = Vec::new();
                    for (pos, client) in &mut state.clients.iter_mut().enumerate() {
                        if let Err(e) = client.send(msg.clone()).await {
                            // Do not let a single monitor affect the larger system
                            println!("Failed to write to monitor: {}", e);
                            client_errors.push(pos);
                        }
                    }
                    // Remove clients with errors, starting from the biggest ID
                    // to not change indices mid-loop
                    for failed_client in client_errors.iter().rev() {
                        let _sink = state.clients.remove(*failed_client);
                    }
                } else {
                    panic!("Lost end of monitor channel");
                }
            }
        });

        Ok(monitor)
    }

    pub async fn get_channel_endpoint(&mut self) -> Option<Sender<Bytes>> {
        self.tx.take()
    }

    pub async fn process(self) -> Result<(), MirrorError> {
        let listen_to = format!("0.0.0.0:{}", self.port);
        let socket = TcpListener::bind(&listen_to).await?;
        println!("Listening on {}", &listen_to);

        loop {
            if let Ok((stream, address)) = socket.accept().await {
                println!("Connection from: {}", address);
                tokio::spawn(handle_client(stream, self.shared.clone()));
            }
        }
    }
}

async fn handle_client(stream: TcpStream, shared: Arc<Mutex<Shared>>) -> Result<(), MirrorError> {
    let client_addr = &stream.peer_addr().unwrap();
    println!("Connection from client {:?}", &client_addr);
    let bytes = Framed::new(stream, BytesCodec::new());
    let (tx, mut rx) = bytes.split();

    {
        let mut state = shared.lock().await;
        state.clients.push(tx);
    }

    loop {
        match rx.next().await {

            Some(Ok(_msg)) => { /* Ignore anything sent by the monitors */ },
            Some(Err(e)) => {
                return Err(MirrorError::from(e));
            },
            None => {
                println!("Client {:?} disconnected", &client_addr);
                return Ok(())
            }
        }
    }
}