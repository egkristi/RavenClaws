//! OpenTelemetry tracing integration for RavenClaw
//!
//! Provides opt-in distributed tracing via OpenTelemetry. When configured,
//! RavenClaw exports traces to an OTLP-compatible collector (e.g., Jaeger,
//! Grafana Tempo, SigNoz, or a self-hosted OpenTelemetry Collector).
//!
//! # Configuration
//!
//! | Env var | CLI flag | Default | Description |
//! |---|---|---|---|
//! | `RAVENCLAW_OTEL_ENDPOINT` | `--otel-endpoint` | `http://localhost:4317` | OTLP gRPC endpoint |
//! | `RAVENCLAW_OTEL_SERVICE_NAME` | `--otel-service-name` | `ravenclaw` | Service name for traces |
//! | `RAVENCLAW_OTEL_DISABLED` | `--otel-disabled` | `false` | Disable OpenTelemetry entirely |
//!
//! # Usage
//!
//! ```ignore
//! use crate::telemetry;
//!
//! let guard = telemetry::init_tracing(&config.telemetry)?;
//! // ... application code ...
//! drop(guard); // Flush and shutdown OTel exporter
//! ```

use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[cfg(feature = "otel-grpc")]
use opentelemetry_otlp::WithExportConfig;

use crate::config::TelemetryConfig;

/// Guard that flushes and shuts down the OTel tracer provider on drop.
/// Must be kept alive for the lifetime of the application.
pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.tracer_provider.take() {
            if let Err(e) = provider.shutdown() {
                tracing::warn!(error = %e, "OpenTelemetry tracer provider shutdown failed");
            }
        }
    }
}

/// Initialize OpenTelemetry tracing.
///
/// Returns a `TelemetryGuard` that must be kept alive for the lifetime of the
/// application. When the guard is dropped, the OTel exporter is flushed and
/// shut down gracefully.
///
/// If `config.otel_disabled` is true, this is a no-op and returns an empty guard.
pub fn init_tracing(config: &TelemetryConfig) -> anyhow::Result<TelemetryGuard> {
    if config.otel_disabled {
        tracing::info!("OpenTelemetry tracing is disabled");
        return Ok(TelemetryGuard {
            tracer_provider: None,
        });
    }

    let service_name = config
        .otel_service_name
        .clone()
        .unwrap_or_else(|| "ravenclaw".to_string());

    let endpoint = config
        .otel_endpoint
        .clone()
        .unwrap_or_else(|| "http://localhost:4317".to_string());

    let resource = Resource::builder()
        .with_attribute(opentelemetry::KeyValue::new(
            "service.name",
            service_name.clone(),
        ))
        .with_attribute(opentelemetry::KeyValue::new(
            "service.version",
            env!("CARGO_PKG_VERSION"),
        ))
        .build();

    // Build the OTLP exporter or stdout exporter based on available features
    #[cfg(feature = "otel-grpc")]
    let tracer_provider = {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create OTLP span exporter: {}", e))?;

        SdkTracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(exporter)
            .build()
    };

    #[cfg(not(feature = "otel-grpc"))]
    let tracer_provider = {
        // Fallback: use stdout exporter if available, otherwise no-op
        #[cfg(feature = "otel-stdout")]
        {
            let exporter = opentelemetry_stdout::SpanExporter::default();
            SdkTracerProvider::builder()
                .with_resource(resource)
                .with_simple_exporter(exporter)
                .build()
        }
        #[cfg(not(feature = "otel-stdout"))]
        {
            tracing::warn!(
                "OpenTelemetry tracing requested but no exporter feature enabled. \
                 Enable 'otel-grpc' or 'otel-stdout' feature."
            );
            SdkTracerProvider::builder().with_resource(resource).build()
        }
    };

    let tracer = tracer_provider.tracer("ravenclaw");
    let telemetry_layer = OpenTelemetryLayer::new(tracer);

    // Register the OTel layer on top of the existing subscriber.
    // We use a try_init approach since the subscriber may already be registered
    // (e.g., from main.rs). If it fails, we log a warning and continue.
    let registry = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "ravenclaw=info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .with(telemetry_layer);

    if registry.try_init().is_err() {
        tracing::warn!(
            "Tracing subscriber already initialized — OpenTelemetry layer not registered. \
             Set RAVENCLAW_OTEL_DISABLED=true if this is unexpected."
        );
    }

    tracing::info!(
        endpoint = %endpoint,
        service = %service_name,
        "OpenTelemetry tracing initialized"
    );

    Ok(TelemetryGuard {
        tracer_provider: Some(tracer_provider),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert!(config.otel_endpoint.is_none());
        assert!(config.otel_service_name.is_none());
        assert!(!config.otel_disabled);
    }

    #[test]
    fn test_telemetry_config_disabled() {
        let config = TelemetryConfig {
            otel_disabled: true,
            ..TelemetryConfig::default()
        };
        let guard = init_tracing(&config).unwrap();
        assert!(guard.tracer_provider.is_none());
    }

    #[test]
    fn test_telemetry_guard_drop_no_panic() {
        let guard = TelemetryGuard {
            tracer_provider: None,
        };
        drop(guard); // Should not panic
    }

    #[test]
    fn test_telemetry_config_custom() {
        let config = TelemetryConfig {
            otel_endpoint: Some("http://jaeger:4317".to_string()),
            otel_service_name: Some("my-ravenclaw".to_string()),
            otel_disabled: false,
        };
        assert_eq!(config.otel_endpoint.as_deref(), Some("http://jaeger:4317"));
        assert_eq!(config.otel_service_name.as_deref(), Some("my-ravenclaw"));
    }
}
