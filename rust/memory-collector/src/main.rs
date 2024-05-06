use std::net;

use memory_collector::*;

const PORT: u16 = 50051;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let host = net::IpAddr::V6(net::Ipv6Addr::LOCALHOST);
    let address = net::SocketAddr::new(host, PORT);
    serve(&State::new(), address).await
}
