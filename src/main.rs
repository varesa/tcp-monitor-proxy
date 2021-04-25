mod errors;
mod mirror;
mod bidirectional_channel;

use crate::errors::ApplicationError::{self, NameResolutionError};
use crate::mirror::Mirror;
use std::net::ToSocketAddrs;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long)]
    remote: String,

    #[structopt(short, long, default_value = "50000")]
    localport: u16,

    #[structopt(short, long, default_value = "50001")]
    mirrorport: u16,
}

#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    let opt = Opt::from_args();

    let remote = opt.remote
        .to_socket_addrs()?.next()
        .ok_or(NameResolutionError(format!("No addresses returned for {}", opt.remote)))
        .unwrap();

    let mirror = Mirror::try_new(opt.localport, opt.mirrorport, remote).await?;
    mirror.wait().await?;
    Ok(())
}
