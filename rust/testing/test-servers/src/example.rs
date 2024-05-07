use std::io;
use std::net;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::process;
use tokio::time::sleep;

/// A reference to a running example server process.
///
/// The process will be terminated on drop.
pub struct Example {
    /// The name of the server.
    pub name: String,
    /// The address that the server is listening on.
    pub address: net::SocketAddr,
    /// The running process.
    child: process::Child,
}

impl Example {
    /// The server's URL, constructed from its address.
    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }
}

// This implementation is necessarily complicated.
impl Drop for Example {
    fn drop(&mut self) {
        // The drop is synchronous, but we need to do things asynchronously,
        // so we block.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                // If the child ID is `None`, it's already been stopped.
                if let Some(id) = self.child.id() {
                    // First, we issue the `SIGTERM` signal.
                    nix::sys::signal::kill(
                        nix::unistd::Pid::from_raw(id.try_into().unwrap()),
                        nix::sys::signal::Signal::SIGTERM,
                    )
                    .unwrap_or(());
                    // We wait 1 second for the process to stop on its own.
                    // If it doesn't, we kill it (with `SIGKILL`).
                    tokio::select! {
                        _ = self.child.wait() => {},
                        () = sleep(Duration::from_secs(1)) => self.child.kill().await.unwrap_or(()),
                    }
                }
            });
        });
    }
}

/// Starts the named example server, specifying an OpenTelemetry endpoint.
/// The server will be built and run through `cargo` on demand.
///
/// This function finds a free port and assigns it to the server process through
/// the `PORT` variable, then waits for the server to start on that port.
///
/// On success, the return value contains the process information. The process
/// will automatically be terminated when the value is dropped.
///
/// This will return an error on build failure, or if the server fails to start
/// on the specified port.
pub async fn start_example(name: &str, otel_endpoint: &str) -> anyhow::Result<Example> {
    process::Command::new("cargo")
        .args(["build", "--quiet", "--example", name])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .spawn()?
        .wait()
        .await?;

    let address = find_free_port()?;
    let port = address.port();
    let child = process::Command::new("cargo")
        .args(["run", "--quiet", "--example", name])
        .env("OTEL_EXPORTER_OTLP_ENDPOINT", otel_endpoint)
        .env("OTEL_BSP_SCHEDULE_DELAY", "100") // send batches very quickly
        .env("PORT", port.to_string())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .spawn()?;
    let wrapped = Example {
        name: name.to_owned(),
        address,
        child,
    };
    wait_for_port(address).await?;
    Ok(wrapped)
}

fn find_free_port() -> io::Result<net::SocketAddr> {
    tokio::task::block_in_place(|| {
        let address = net::SocketAddr::new(net::IpAddr::V6(net::Ipv6Addr::LOCALHOST), 0);
        let listener = net::TcpListener::bind(address)?;
        listener.local_addr()
    })
}

async fn wait_for_port(address: net::SocketAddr) -> anyhow::Result<()> {
    tokio::select! {
        () = repeatedly_try_to_connect(address) => Ok(()),
        () = sleep(Duration::from_secs(1)) => Err(anyhow::anyhow!("Gave up waiting for {address} to open.")),
    }
}

async fn repeatedly_try_to_connect(address: net::SocketAddr) {
    loop {
        let result = TcpStream::connect(address).await;
        if result.is_ok() {
            return;
        }
    }
}
