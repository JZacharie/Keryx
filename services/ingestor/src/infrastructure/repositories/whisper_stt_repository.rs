use async_trait::async_trait;
use anyhow::Result;
use std::path::PathBuf;
use crate::domain::ports::stt_repository::{STTRepository, TranscriptionResult, TranscriptionSegment};
use reqwest::multipart;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

pub struct WhisperSTTRepository {
    client: reqwest::Client,
    base_url: String,
}

impl WhisperSTTRepository {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl STTRepository for WhisperSTTRepository {
    async fn transcribe(&self, audio_path: &PathBuf) -> Result<TranscriptionResult> {
        let file = File::open(audio_path).await?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);

        let part = multipart::Part::stream(body)
            .file_name(audio_path.file_name().unwrap_or_default().to_string_lossy().into_owned())
            .mime_str("audio/wav")?;

        let form = multipart::Form::new().part("audio_file", part);

        let response = self.client.post(format!("{}/asr", self.base_url))
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Whisper API failed with status: {}", response.status()));
        }

        let result: WhisperResponse = response.json().await?;

        let segments = result.segments.into_iter().map(|s| {
            TranscriptionSegment {
                start: s.start,
                end: s.end,
                text: s.text,
            }
        }).collect();

        Ok(TranscriptionResult { segments })
    }
}

#[derive(serde::Deserialize)]
struct WhisperResponse {
    segments: Vec<WhisperSegment>,
}

#[derive(serde::Deserialize)]
struct WhisperSegment {
    start: f64,
    end: f64,
    text: String,
}
