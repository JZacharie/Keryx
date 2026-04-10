use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    pub id: Uuid,
    pub source_url: String,
    pub target_langs: Vec<String>,
    pub status: JobStatus,
    pub style_config: StyleConfig,
    pub assets_map: Vec<SlideAsset>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum JobStatus {
    Pending,
    Downloading,
    Analyzing,
    Transcribing,
    Translating,
    GeneratingVisuals,
    CloningVoice,
    Composing,
    Completed,
    Failed(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StyleConfig {
    pub prompt: String,
    pub lora: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SlideAsset {
    pub slide_index: u32,
    pub original_frame: String,
    pub timestamp: f64,
    pub translations: HashMap<String, TranslationAsset>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranslationAsset {
    pub text: String,
    pub styled_image: Option<String>,
    pub audio: Option<String>,
    pub duration: f64,
}
