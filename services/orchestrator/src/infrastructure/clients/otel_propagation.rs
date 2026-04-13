/// Propagation W3C TraceContext pour les appels HTTP sortants.
///
/// Injecte le header `traceparent` (et `tracestate` si présent) dans les requêtes
/// reqwest afin que les services downstream puissent participer au même trace distribué.
/// C'est ce mécanisme qui génère le **service map** dans OpenObserve.
use opentelemetry::propagation::Injector;
use reqwest::RequestBuilder;
use std::collections::HashMap;

/// Wrapper pour injecter les headers OTel dans une map de strings.
struct HeaderInjector<'a>(&'a mut HashMap<String, String>);

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_lowercase(), value);
    }
}

/// Injecte les headers de propagation W3C (`traceparent`, `tracestate`) dans un `RequestBuilder`.
///
/// À appeler sur chaque requête HTTP sortante vers les services Keryx downstream.
pub fn inject_trace_context(request: RequestBuilder) -> RequestBuilder {
    let mut headers = HashMap::new();

    opentelemetry::global::get_text_map_propagator(|propagator| {
        // Utilise le contexte du span actif (fourni par tracing-opentelemetry)
        let ctx = opentelemetry::Context::current();
        propagator.inject_context(&ctx, &mut HeaderInjector(&mut headers));
    });

    // Injecte les headers dans la requête
    let mut builder = request;
    for (key, value) in headers {
        if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(header_value) = reqwest::header::HeaderValue::from_str(&value) {
                builder = builder.header(header_name, header_value);
            }
        }
    }

    builder
}
