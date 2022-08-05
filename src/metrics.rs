use hyper::{
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use log::info;
use prometheus::{gather, register_histogram, Encoder, Histogram, TextEncoder};
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
            let client_name = s.split("client-name\": \"").collect::<Vec<_>>()[1]
                .split("\", \"user-agent")
                .collect::<Vec<_>>()[0]
                .to_string();
            let func_name = s.split("Service/").collect::<Vec<_>>()[1]
                .split(", version:")
                .collect::<Vec<_>>()[0]
                .to_string();
            let key = (client_name.clone(), func_name.clone());

            let histogram = self.metrics_data.entry(key).or_insert_with(|| {
                register_histogram!(
                    format!("{}_to_{}", client_name, func_name),
                    "request latencies in milliseconds(ms)",
                    self.buckets.clone(),
                )
                .map_err(|e| {
                    info!(
                        "{} register fail",
                        format!("{}_to_{}", client_name, func_name)
                    );
                    e
                })
                .unwrap()
            });
            let histogram = histogram.clone();

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
    info!("exporting metrics to http://{}", addr);

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
