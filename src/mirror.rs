mod server;
mod client;

use std::net::SocketAddr;
use crate::errors::MirrorError;
use tokio::task::JoinHandle;
use crate::mirror::server::Server;
use crate::mirror::client::Client;
use futures::{SinkExt, StreamExt};

pub struct Mirror {
    server_task: JoinHandle<Result<(), MirrorError>>,
    client_task: JoinHandle<Result<(), MirrorError>>,
    forward_task: JoinHandle<Result<(), MirrorError>>,
}

impl Mirror {
    pub async fn try_new(local_port: u16, _mirror_port: u16, remote: SocketAddr) -> Result<Self, MirrorError> {
        let mut server = Server::try_new(local_port, false).await?;
        let mut server_endpoint = server.get_channel_endpoint().await.unwrap();
        let server_task = tokio::spawn(server.process());

        let mut client = Client::try_new(remote).await?;
        let mut client_endpoint = client.get_channel_endpoint().await.unwrap();
        let client_task = tokio::spawn(client.connect());

        let forward_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = server_endpoint.rx.next() => {
                        println!("C->S: {:?}", msg);
                        client_endpoint.tx.send(msg).await?;
                    }
                    Some(msg) = client_endpoint.rx.next() => {
                        println!("S->C: {:?}", msg);
                        server_endpoint.tx.send(msg).await?;
                    }
                }
            }
        });

        Ok(Mirror {
            server_task,
            client_task,
            forward_task,
        })
    }

    pub async fn wait(self) -> Result<(), MirrorError> {
        let results = futures::future::join_all(vec![self.server_task, self.client_task, self.forward_task]).await;
        for result in results {
            result??;
        }
        Ok(())
    }
}
