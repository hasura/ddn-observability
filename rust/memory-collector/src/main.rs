use std::net;

use tokio::net::TcpListener;

use memory_collector::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let address = net::SocketAddr::new(net::IpAddr::V6(net::Ipv6Addr::LOCALHOST), 50051);
    let listener = TcpListener::bind(address).await?;
    serve(&State::new(), listener).await?;
    Ok(())
}
