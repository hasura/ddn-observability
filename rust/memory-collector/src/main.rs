use std::net;

use memory_collector::serve;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = net::SocketAddr::new(net::IpAddr::V6(net::Ipv6Addr::LOCALHOST), 50051);
    serve(address).await?;
    Ok(())
}
