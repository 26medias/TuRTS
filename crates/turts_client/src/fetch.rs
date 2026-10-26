//! HTTP fetch as a Bevy task: native uses `ureq`, the browser uses `gloo-net`.

use bevy::tasks::{IoTaskPool, Task};

/// A fetch in progress. Poll it each frame with [`poll`].
pub struct FetchTask(Task<Result<Vec<u8>, String>>);

/// Start a GET of `url` on the IO task pool. The task pool must exist, which the Bevy task pool
/// plugin sets up. The task yields the response body, or an error text that names the URL.
pub fn fetch_bytes(url: &str) -> FetchTask {
    let url = url.to_string();
    FetchTask(IoTaskPool::get().spawn(get(url)))
}

/// The result of the fetch if it has finished, else `None`. After a `Some`, do not poll again.
pub fn poll(task: &mut FetchTask) -> Option<Result<Vec<u8>, String>> {
    bevy::tasks::futures_lite::future::block_on(bevy::tasks::poll_once(&mut task.0))
}

#[cfg(not(target_family = "wasm"))]
async fn get(url: String) -> Result<Vec<u8>, String> {
    let fail = |error: ureq::Error| format!("cannot fetch {url}: {error}");
    let mut response = ureq::get(&url).call().map_err(fail)?;
    response.body_mut().read_to_vec().map_err(fail)
}

#[cfg(target_family = "wasm")]
async fn get(url: String) -> Result<Vec<u8>, String> {
    let fail = |error: gloo_net::Error| format!("cannot fetch {url}: {error}");
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(fail)?;
    if !response.ok() {
        return Err(format!(
            "cannot fetch {url}: HTTP status {}",
            response.status()
        ));
    }
    response.binary().await.map_err(fail)
}
