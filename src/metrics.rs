use hyper::{
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use log::{info, warn};
use prometheus::{gather, register_histogram, Encoder, Histogram, TextEncoder};
use regex::Regex;
use std::task::{Context, Poll};
use std::time::Instant;
use std::{collections::HashMap, convert::Infallible};
use tonic::body::BoxBody;
use tower::{Layer, Service};

#[derive(Debug, Clone)]
pub struct MiddlewareLayer {
    buckets: Vec<f64>,
}

impl MiddlewareLayer {
    pub fn new(buckets: Vec<f64>) -> Self {
        MiddlewareLayer { buckets }
    }
}

impl<S> Layer<S> for MiddlewareLayer {
    type Service = MetricsData<S>;

    fn layer(&self, service: S) -> Self::Service {
        MetricsData {
            inner: service,
            metrics_data: HashMap::new(),
            buckets: self.buckets.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsData<S> {
    inner: S,
    metrics_data: HashMap<(String, String), Histogram>,
    buckets: Vec<f64>,
}

impl<S> Service<Request<Body>> for MetricsData<S>
where
    S: Service<Request<Body>, Response = Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let s = format!("{:?}", req);
        if s.contains("client-name") {
            let re = Regex::new(r"(Service/)(.+)(, version)(.+)(client-name\u0022: \u0022)(.+)(\u0022, \u0022user-agent)").unwrap();
            let caps = re.captures(&s).unwrap();
            let func_name = caps.get(2).unwrap().as_str();
            let client_name = caps.get(6).unwrap().as_str();
            let key = (client_name.to_string(), func_name.to_string());

            if !self.metrics_data.contains_key(&key) {
                match register_histogram!(
                    format!("{}_to_{}", client_name, func_name),
                    "request latencies in milliseconds(ms)",
                    self.buckets.clone(),
                ) {
                    Ok(histogram) => {
                        info!(
                            "register histogram {} succeeded",
                            format!("{}_to_{}", client_name, func_name)
                        );
                        self.metrics_data.insert(key.clone(), histogram);
                    }
                    Err(e) => {
                        warn!(
                            "register histogram {} failed with error: {}, ignored metrics",
                            format!("{}_to_{}", client_name, func_name),
                            e.to_string()
                        );
                        return Box::pin(async move {
                            let response = inner.call(req).await?;
                            Ok(response)
                        });
                    }
                }
            }

            let histogram = if let Some(h) = self.metrics_data.get(&key) {
                h.to_owned()
            } else {
                warn!(
                    "register histogram {} succeeded but get it failed, ignored metrics",
                    format!("{}_to_{}", client_name, func_name)
                );
                return Box::pin(async move {
                    let response = inner.call(req).await?;
                    Ok(response)
                });
            };

            return Box::pin(async move {
                let started = Instant::now();

                let response = inner.call(req).await?;

                let elapsed = started.elapsed().as_secs_f64() * 1000f64;
                histogram.observe(elapsed);

                Ok(response)
            });
        }
        Box::pin(async move {
            let response = inner.call(req).await?;
            Ok(response)
        })
    }
}

pub async fn run_metrics_exporter(
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let make_svc =
        make_service_fn(move |_conn| async move { Ok::<_, Infallible>(service_fn(serve_req)) });

    let addr = ([0, 0, 0, 0], port).into();
    let server = Server::bind(&addr).serve(make_svc);
    info!("exporting metrics to http://{}/metrics", addr);

    server.await?;

    Ok(())
}

async fn serve_req(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let mut buffer = vec![];
            let encoder = TextEncoder::new();
            let metric_families = gather();
            encoder.encode(&metric_families, &mut buffer).unwrap();

            Response::builder()
                .status(200)
                .header(CONTENT_TYPE, encoder.format_type())
                .body(Body::from(buffer))
                .unwrap()
        }
        _ => Response::builder()
            .status(404)
            .body(Body::from(
                "
            default:\n
            /60000/metrics for network\n
            /60001/metrics for consensus\n
            /60002/metrics for executor\n
            /60003/metrics for storage\n
            /60004/metrics for controller\n
            /60005/metrics for crypto\n
            ",
            ))
            .unwrap(),
    };

    Ok(response)
}
