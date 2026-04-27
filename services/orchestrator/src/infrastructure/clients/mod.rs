pub mod texts_translation;
pub mod extractor;
pub mod dewatermark;
pub mod voice_extractor;
pub mod voice_cloner;
pub mod video_composer;
pub mod video_generator;
pub mod diffusion_engine;
pub mod pptx_builder;
pub mod otel_propagation;

use std::time::Duration;
use anyhow::Result;
use tokio::time::sleep;

pub async fn execute_with_retry<F, Fut, T>(mut f: F, max_retries: u32) -> Result<T> 
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(res) => return Ok(res),
            Err(e) if retries < max_retries => {
                retries += 1;
                let wait_time = Duration::from_secs(2u64.pow(retries));
                tracing::warn!("Request failed ({}): {}. Retrying in {:?}...", retries, e, wait_time);
                sleep(wait_time).await;
            }
            Err(e) => return Err(e),
        }
    }
}
