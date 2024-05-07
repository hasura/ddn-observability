use std::future::Future;
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
/// the `PORT` variable. It then waits for the server to start, first by
/// attempting to connect to the port, and then making a request to the
/// "/health" path. It only returns once these two checks have passed.
///
/// On success, the return value contains the process information. The process
/// will automatically be terminated when the value is dropped.
///
/// This will return an error on build failure, or if the server fails to start
/// on the specified port.
pub async fn start_example(
    name: &str,
    otel_endpoint: &str,
    environment: Vec<(&str, &str)>,
) -> anyhow::Result<Example> {
    process::Command::new("cargo")
        .args(["build", "--example", name])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .spawn()?
        .wait()
        .await?;

    let address = find_free_port()?;
    let port = address.port();
    println!("Starting {name} on {address}...");
    let child = process::Command::new("cargo")
        .args(["run", "--example", name])
        .envs(environment)
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
    wait_until_healthy(address).await?;
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
    let _ = wait_for(
        || TcpStream::connect(address),
        Duration::from_secs(1),
        &format!("opening {address}"),
    )
    .await?;
    Ok(())
}

async fn wait_until_healthy(address: net::SocketAddr) -> anyhow::Result<()> {
    let url = format!("http://{address}/health");
    wait_for(
        || async {
            reqwest::get(&url).await.map_err(|_| ()).and_then(|r| {
                if r.status().is_success() {
                    Ok(())
                } else {
                    Err(())
                }
            })
        },
        Duration::from_secs(1),
        &format!("checking for health of {address}"),
    )
    .await
}

async fn wait_for<T, E, F, Action>(
    action: Action,
    timeout: Duration,
    description: &str,
) -> anyhow::Result<T>
where
    F: Future<Output = Result<T, E>>,
    Action: Fn() -> F,
{
    tokio::select! {
        value = repeatedly(action) => Ok(value),
        () = sleep(timeout) => Err(anyhow::anyhow!("Gave up waiting: {description}")),
    }
}

async fn repeatedly<T, E, F, Action>(action: Action) -> T
where
    F: Future<Output = Result<T, E>>,
    Action: Fn() -> F,
{
    loop {
        if let Ok(value) = action().await {
            return value;
        }
    }
}
