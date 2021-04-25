use tokio::net::TcpStream;
use crate::errors::MirrorError;
use crate::bidirectional_channel::{ChannelEndpoint, bidirectional_channel};
use tokio_util::codec::{Framed, BytesCodec};
use futures::{StreamExt, SinkExt};
use bytes::Bytes;
use std::net::SocketAddr;

pub struct Client {
    remote: SocketAddr,
    channel_local_endpoint:  ChannelEndpoint<Bytes>,
    channel_remote_endpoint: Option<ChannelEndpoint<Bytes>>,
}

impl Client {
    pub async fn try_new(remote: SocketAddr) -> Result<Self, MirrorError> {
        let (endpoint_a, endpoint_b) = bidirectional_channel();

        Ok(Client {
            remote,
            channel_local_endpoint: endpoint_a,
            channel_remote_endpoint: Some(endpoint_b),
        })
    }

    pub async fn get_channel_endpoint(&mut self) -> Option<ChannelEndpoint<Bytes>> {
        self.channel_remote_endpoint.take()
    }

    pub async fn connect(mut self) -> Result<(), MirrorError> {
        loop {
            let connection = TcpStream::connect(self.remote).await?;
            println!("Connected to remote {:?}", &connection.peer_addr().unwrap());

            let mut bytes = Framed::new(connection, BytesCodec::new());

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
                            None => break,
                        }
                    }
                    Some(msg) = self.channel_local_endpoint.rx.next() => {
                        bytes.send(msg).await?;
                    }
                }
            }
            println!("Disconnect from remote");
        }
    }
}