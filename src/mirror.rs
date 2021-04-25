use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use crate::errors::MirrorError;
use tokio::task::JoinHandle;
use crate::bidirectional_channel::{ChannelEndpoint, bidirectional_channel};
use tokio_util::codec::{Framed, BytesCodec};
use futures::{StreamExt, SinkExt};
use bytes::Bytes;

struct Server {
    port: u16,
    read_only: bool,
    channel_local_endpoint:  ChannelEndpoint<Bytes>,
    channel_remote_endpoint: Option<ChannelEndpoint<Bytes>>,
}

impl Server {
    pub async fn try_new(port: u16, read_only: bool) -> Result<Self, MirrorError> {
        let (endpoint_a, endpoint_b) = bidirectional_channel();

        Ok(Server {
            port,
            read_only,
            channel_local_endpoint: endpoint_a,
            channel_remote_endpoint: Some(endpoint_b),
        })
    }

    pub async fn get_channel_endpoint(&mut self) -> Option<ChannelEndpoint<Bytes>> {
        self.channel_remote_endpoint.take()
    }

    async fn handle_client(&mut self, stream: TcpStream) -> Result<(), MirrorError> {
        let mut bytes = Framed::new(stream, BytesCodec::new());

        loop {
            tokio::select! {
                    Some(Ok(msg)) = bytes.next() => {
                        if !self.read_only {
                            println!("R: {:?}", msg);
                            self.channel_local_endpoint.tx.send(msg.freeze()).await?;
                        }
                    }
                    Some(msg) = self.channel_local_endpoint.rx.next() => {
                        println!("T: {:?}", msg);
                        bytes.send(msg).await?;
                    }
                }
        }
    }

    pub async fn process(mut self) -> Result<(), MirrorError> {
        let listen_to = format!("0.0.0.0:{}", self.port);
        loop {
            let socket = TcpListener::bind(&listen_to).await?;
            println!("Listening on {}", &listen_to);

            let connection = socket.accept().await;
            if let Ok((stream, address)) = connection {
                drop(socket);
                println!("Connection from: {}", address);

                self.handle_client(stream).await?;
            }
        }
    }
}

pub struct Mirror {
    server_task: JoinHandle<Result<(), MirrorError>>
}

impl Mirror {
    pub async fn try_new(local_port: u16, _mirror_port: u16, _remote: SocketAddr) -> Result<Self, MirrorError> {
        let mut server = Server::try_new(local_port, false).await?;
        let _server_endpoint = server.get_channel_endpoint().await.unwrap();
        let server_task = tokio::spawn(server.process());
        Ok(Mirror {
            server_task
        })
    }

    pub async fn wait(self) -> Result<(), MirrorError> {
        self.server_task.await??;
        Ok(())
    }
}
