use std::net;

use opentelemetry_semantic_conventions as semcov;
use tokio::net::TcpListener;

use ddn_tracing::*;
use tracing::{info_span, Instrument};

#[tokio::test(flavor = "multi_thread")]
async fn defines_resource_attributes() -> anyhow::Result<()> {
    let service_name = "test-service";
    let service_version = "1.2.3";

    let server_listener = TcpListener::bind((net::IpAddr::V6(net::Ipv6Addr::LOCALHOST), 0)).await?;
    let server_address = server_listener.local_addr()?;
    let server_endpoint = format!("http://[{}]:{}", server_address.ip(), server_address.port());
    let collector_state = memory_collector::State::new();
    let _collector_server =
        memory_collector::serve_in_background(&collector_state, server_listener)?;

    let value = {
        let _global_tracing = init_tracing(Some(&server_endpoint), service_name, service_version)
            .map_err(|e| anyhow::anyhow!(e))?;

        async { Ok::<_, std::convert::Infallible>(7) }
            .instrument(info_span!("defines_resource_attributes"))
            .await?
    };

    assert_eq!(value, 7);

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
                        service_name.to_string()
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
                        service_version.to_string()
                    ))
                })
            })
        );
    } else {
        anyhow::bail!("There should be exactly one resource span set.\nGot: {spans:?}");
    }

    Ok(())
}
