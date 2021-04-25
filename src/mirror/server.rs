use tokio::net::{TcpListener, TcpStream};
use crate::errors::MirrorError;
use crate::bidirectional_channel::{ChannelEndpoint, bidirectional_channel};
use tokio_util::codec::{Framed, BytesCodec};
use futures::{StreamExt, SinkExt};
use bytes::Bytes;

pub struct Server {
    port: u16,
    channel_local_endpoint:  ChannelEndpoint<Bytes>,
    channel_remote_endpoint: Option<ChannelEndpoint<Bytes>>,
}

impl Server {
    pub async fn try_new(port: u16) -> Result<Self, MirrorError> {
        let (endpoint_a, endpoint_b) = bidirectional_channel();

        Ok(Server {
            port,
            channel_local_endpoint: endpoint_a,
            channel_remote_endpoint: Some(endpoint_b),
        })
    }

    pub async fn get_channel_endpoint(&mut self) -> Option<ChannelEndpoint<Bytes>> {
        self.channel_remote_endpoint.take()
    }

    async fn handle_client(&mut self, stream: TcpStream) -> Result<(), MirrorError> {
        println!("Connection from client {:?}", &stream.peer_addr().unwrap());
        let mut bytes = Framed::new(stream, BytesCodec::new());

        loop {
            tokio::select! {
                    result = bytes.next() => {
                        match result {
                            Some(Ok(msg)) => {
                                self.channel_local_endpoint.tx.send(msg.freeze()).await?;
                            }
                            Some(Err(e)) => {
                                return Err(MirrorError::from(e));
                            }
                            None => return Ok(())
                        }
                    }
                    Some(msg) = self.channel_local_endpoint.rx.next() => {
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