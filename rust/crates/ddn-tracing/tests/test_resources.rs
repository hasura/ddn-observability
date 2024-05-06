use opentelemetry_semantic_conventions as semcov;

#[tokio::test(flavor = "multi_thread")]
async fn defines_resource_attributes() -> anyhow::Result<()> {
    let collector_state = memory_collector::State::new();
    let collector_server = memory_collector::serve_in_background(&collector_state).await?;

    let example_server =
        test_servers::example::start_example("echo-server", &collector_server.url()).await?;

    let response = reqwest::Client::new()
        .post(example_server.url() + "/echo")
        .body("Hello there!")
        .send()
        .await?;

    let response_body = response.text().await?;

    assert_eq!(response_body, "Hello there!");

    collector_state.wait_for_next_write().await;
    let spans = collector_state.read();
    if let [memory_collector::proto::ResourceSpans {
        resource: Some(memory_collector::proto::Resource { attributes, .. }),
        ..
    }] = &spans[..]
    {
        let service_name_pair = attributes
            .iter()
            .find(|attribute| attribute.key == semcov::resource::SERVICE_NAME);
        assert_eq!(
            service_name_pair,
            Some(&memory_collector::proto::KeyValue {
                key: semcov::resource::SERVICE_NAME.to_string(),
                value: Some(memory_collector::proto::AnyValue {
                    value: Some(memory_collector::proto::any_value::Value::StringValue(
                        example_server.name.clone()
                    ))
                })
            })
        );

        let service_version_pair = attributes
            .iter()
            .find(|attribute| attribute.key == semcov::resource::SERVICE_VERSION);
        assert_eq!(
            service_version_pair,
            Some(&memory_collector::proto::KeyValue {
                key: semcov::resource::SERVICE_VERSION.to_string(),
                value: Some(memory_collector::proto::AnyValue {
                    value: Some(memory_collector::proto::any_value::Value::StringValue(
                        env!("CARGO_PKG_VERSION").to_owned()
                    ))
                })
            })
        );
    } else {
        anyhow::bail!(
            "There should be exactly one resource span set.\nGot {}: {:#?}",
            spans.len(),
            spans
        );
    }

    Ok(())
}
