#[tokio::test(flavor = "multi_thread")]
async fn captures_the_parent() -> anyhow::Result<()> {
    let collector_state = memory_collector::State::new();

    {
        let collector_server = memory_collector::serve_in_background(&collector_state).await?;

        let echo_server =
            test_servers::example::start_example("echo-server", &collector_server.url(), vec![])
                .await?;
        let proxy_server = test_servers::example::start_example(
            "proxy-server",
            &collector_server.url(),
            vec![("TARGET_URL", echo_server.url().as_str())],
        )
        .await?;

        let response = reqwest::Client::new()
            .post(proxy_server.url() + "/echo")
            .body("Ring, ring, ring.")
            .send()
            .await?;

        let response_body = response.text().await?;

        assert_eq!(response_body, "Ring, ring, ring.");
    }

    collector_state.wait_for_next_write().await;
    let spans = collector_state.read();

    Ok(())
}
