//! The bridge between GPUI and the core (docs/desktop-app.md §5.1).
//!
//! The core needs tokio (reqwest and `spawn_blocking` panic outside a runtime) and GPUI's
//! executor isn't tokio, so there is one global multi-threaded runtime. A screen runs a core call
//! with [`spawn_core`] and awaits it inside `cx.spawn(async move |this, cx| …)`, then applies the
//! result with `this.update(cx, …)`.
//!
//! [`HttpClient`] gives GPUI's `img(url)` the same runtime and the core's User-Agent, so covers
//! load (the app starts with GPUI's null client, which loads nothing).

use std::future::Future;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use futures::FutureExt;
use futures::future::BoxFuture;
use gpui_kit::http_client::{self, AsyncBody, Response, Url, http};

/// The app's one tokio runtime, started on first use.
pub fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("librespydus-core")
            .enable_all()
            .build()
            .expect("the tokio runtime starts")
    })
}

/// A handle to [`runtime`], for code that spawns on it directly.
pub fn handle() -> tokio::runtime::Handle {
    runtime().handle().clone()
}

/// Runs `future` on the core's runtime and returns a future, awaitable from any executor (GPUI's
/// included), that resolves to its output. A panic in `future` resumes in the caller.
pub fn spawn_core<F>(future: F) -> impl Future<Output = F::Output> + Send + 'static
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let task = runtime().spawn(future);
    async move {
        match task.await {
            Ok(value) => value,
            Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
            Err(error) => panic!("core task cancelled: {error}"),
        }
    }
}

/// Sleeps on the core's runtime (a demo backend's delays), awaitable from any executor.
pub fn sleep(duration: Duration) -> impl Future<Output = ()> + Send + 'static {
    // Made on the runtime: a tokio timer needs its reactor when it is created.
    spawn_core(async move { tokio::time::sleep(duration).await })
}

/// GPUI's HTTP client (`img(url)`), on reqwest and the core's runtime. Sends the core's
/// User-Agent; follows redirects; a whole response body is read before it is handed over.
pub struct HttpClient {
    client: reqwest::Client,
    user_agent: http::HeaderValue,
}

impl HttpClient {
    pub fn new() -> Arc<Self> {
        let agent = librespydus_core::user_agent();
        let client = reqwest::Client::builder()
            .user_agent(agent.clone())
            .timeout(Duration::from_secs(30))
            .gzip(true)
            .build()
            .expect("the image HTTP client builds");
        Arc::new(Self {
            client,
            user_agent: http::HeaderValue::from_str(&agent)
                .unwrap_or_else(|_| http::HeaderValue::from_static("LibreSpydus")),
        })
    }
}

impl http_client::HttpClient for HttpClient {
    fn user_agent(&self) -> Option<&http::HeaderValue> {
        Some(&self.user_agent)
    }

    fn proxy(&self) -> Option<&Url> {
        None
    }

    fn send(&self, request: http::Request<AsyncBody>) -> BoxFuture<'static, anyhow::Result<Response<AsyncBody>>> {
        let client = self.client.clone();
        let (parts, body) = request.into_parts();
        let body = match body.0 {
            http_client::Inner::Empty => Vec::new(),
            http_client::Inner::Bytes(cursor) => cursor.into_inner().to_vec(),
            // GPUI only streams request bodies for uploads, which the app never makes.
            http_client::Inner::AsyncReader(_) => {
                return async { Err(anyhow::anyhow!("streamed request bodies aren't supported")) }.boxed();
            }
        };
        spawn_core(async move {
            let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())?;
            let mut builder = client.request(method, parts.uri.to_string());
            for (name, value) in &parts.headers {
                builder = builder.header(name.as_str(), value.as_bytes());
            }
            if !body.is_empty() {
                builder = builder.body(body);
            }
            let response = builder.send().await?;
            let mut out = Response::builder().status(response.status().as_u16());
            for (name, value) in response.headers() {
                out = out.header(name.as_str(), value.as_bytes());
            }
            let bytes = response.bytes().await?;
            Ok(out.body(AsyncBody::from(bytes.to_vec()))?)
        })
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_works_from_outside_the_runtime() {
        futures::executor::block_on(sleep(Duration::from_millis(1)));
    }

    #[test]
    fn spawn_core_runs_on_the_runtime_and_returns_its_value() {
        let value = futures::executor::block_on(spawn_core(async {
            tokio::time::sleep(Duration::from_millis(1)).await;
            tokio::runtime::Handle::try_current().is_ok()
        }));
        assert!(value);
    }
}
