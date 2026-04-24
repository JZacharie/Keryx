use serde::{Deserialize, Serialize};
use crate::infrastructure::clients::extractor::ExtractResponse;
use crate::infrastructure::clients::voice_extractor::{TranscribeResponse, Segment};
use crate::infrastructure::clients::video_composer::DetectSlidesResponse;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct JobTrackingData {
    pub url_hash: String,
    pub source_url: String,
    pub extraction: Option<ExtractResponse>,
    pub transcription: Option<TranscribeResponse>,
    pub slide_detection: Option<DetectSlidesResponse>,
    pub cleaned_slides: Vec<CleanedSlide>,
    pub translation_segments: Option<Vec<Segment>>,
    pub cloned_audio_urls: Vec<String>,
    pub final_audio_url: Option<String>,
    pub final_video_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CleanedSlide {
    pub index: u32,
    pub original_url: String,
    pub cleaned_url: String,
    pub timestamp: f64,
}
