/// Telemetry — OpenTelemetry APM + Service Map → OpenObserve
///
/// Initialise le TracerProvider et MeterProvider via OTLP/gRPC,
/// et branche le tout sur `tracing-subscriber` pour une intégration transparente.
///
/// Usage :
/// ```rust
/// let _guard = telemetry::init_telemetry("keryx-orchestrator", endpoint, auth_token);
/// // _guard doit vivre jusqu'à la fin du processus (Drop = flush + shutdown)
/// ```
use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    metrics::{
        reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
        MeterProviderBuilder, PeriodicReader, SdkMeterProvider,
    },
    runtime,
    trace::{BatchConfig, RandomIdGenerator, Sampler, Tracer},
    Resource,
};
use opentelemetry_semantic_conventions::resource::{
    DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION,
};
use opentelemetry_semantic_conventions::SCHEMA_URL;
use tonic::metadata::MetadataMap;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Guard qui flush et shutdown proprement les providers OTel à la fin du processus.
pub struct OtelGuard {
    meter_provider: SdkMeterProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(err) = self.meter_provider.shutdown() {
            eprintln!("[otel] meter_provider shutdown error: {err:?}");
        }
        opentelemetry::global::shutdown_tracer_provider();
    }
}

/// Construit la Resource OTel avec les attributs sémantiques standards.
fn build_resource(service_name: &str) -> Resource {
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, service_name.to_owned()),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            KeyValue::new(DEPLOYMENT_ENVIRONMENT, "production"),
        ],
        SCHEMA_URL,
    )
}

/// Construit les metadata gRPC (authorization + organisation OpenObserve).
fn build_grpc_metadata(auth_token: &str) -> MetadataMap {
    let mut map = MetadataMap::with_capacity(3);
    map.insert(
        "authorization",
        auth_token
            .parse()
            .expect("[otel] invalid OTEL_AUTH_TOKEN format"),
    );
    map.insert("organization", "default".parse().unwrap());
    map.insert("stream-name", "default".parse().unwrap());
    map
}

/// Initialise le MeterProvider OTLP/gRPC (métriques HTTP → OpenObserve).
fn init_meter_provider(resource: Resource, endpoint: &str, auth_token: &str) -> SdkMeterProvider {
    let metadata = build_grpc_metadata(auth_token);

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .with_metadata(metadata)
        .build_metrics_exporter(
            Box::new(DefaultAggregationSelector::new()),
            Box::new(DefaultTemporalitySelector::new()),
        )
        .expect("[otel] failed to build metrics exporter");

    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    opentelemetry::global::set_meter_provider(meter_provider.clone());
    meter_provider
}

/// Initialise le TracerProvider OTLP/gRPC (traces distribuées → OpenObserve).
/// Retourne un `Tracer` prêt à être branché sur `tracing_opentelemetry::OpenTelemetryLayer`.
fn init_tracer(resource: Resource, endpoint: &str, auth_token: &str) -> Tracer {
    let metadata = build_grpc_metadata(auth_token);

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                // 100% des traces — ajuster en prod si trop de volume
                .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(1.0))))
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .with_batch_config(BatchConfig::default())
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint)
                .with_metadata(metadata),
        )
        .install_batch(runtime::Tokio)
        .expect("[otel] failed to install tracer provider");

    opentelemetry::global::set_tracer_provider(provider.clone());

    // Active la propagation W3C TraceContext (traceparent/tracestate) — requis pour le service map
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    provider.tracer("keryx-otel")
}

/// Point d'entrée principal : initialise traces + métriques et branche sur tracing-subscriber.
///
/// Le `OtelGuard` retourné **doit être maintenu en vie** jusqu'à l'arrêt du processus.
pub fn init_telemetry(service_name: &str, endpoint: &str, auth_token: &str) -> OtelGuard {
    let resource = build_resource(service_name);

    let meter_provider = init_meter_provider(resource.clone(), endpoint, auth_token);
    let tracer = init_tracer(resource, endpoint, auth_token);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true),
        )
        .with(MetricsLayer::new(meter_provider.clone()))
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    OtelGuard { meter_provider }
}
