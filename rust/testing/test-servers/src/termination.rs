pub async fn wait_for_termination() {
    // wait for a SIGINT, i.e. a Ctrl+C from the keyboard
    let sigint = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install signal handler");
    };

    // wait for a SIGTERM, i.e. a normal `kill` command
    #[cfg(unix)]
    let sigterm = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await
            .unwrap_or(());
    };

    // block until either of the above happens
    #[cfg(unix)]
    tokio::select! {
        () = sigint => (),
        () = sigterm => (),
    }
    #[cfg(not(unix))]
    tokio::select! {
        () = sigint => (),
    }
}
