use serde::{Deserialize, Serialize};
use crate::infrastructure::clients::extractor::ExtractResponse;
use crate::infrastructure::clients::voice_extractor::{TranscribeResponse, Segment};
use crate::infrastructure::clients::video_composer::DetectSlidesResponse;

use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct JobTrackingData {
    pub url_hash: String,
    pub source_url: String,
    pub extraction: Option<ExtractResponse>,
    pub transcription: Option<TranscribeResponse>,
    pub slide_detection: Option<DetectSlidesResponse>,
    pub cleaned_slides: Vec<CleanedSlide>,
    pub styled_slides: Vec<StyledSlide>,
    
    #[serde(default)]
    pub refined_texts: Vec<String>,
    
    #[serde(default)]
    pub translations: HashMap<String, Vec<Segment>>,
    
    #[serde(default)]
    pub cloned_audios: HashMap<String, Vec<String>>,
    
    #[serde(default)]
    pub cloned_durations: HashMap<String, Vec<f64>>,
    
    #[serde(default)]
    pub final_audios: HashMap<String, String>,
    
    #[serde(default)]
    pub final_videos: HashMap<String, String>,

    pub pptx_url: Option<String>,

    // Backward compatibility
    pub translation_segments: Option<Vec<Segment>>,
    pub cloned_audio_urls: Vec<String>,
    pub final_audio_url: Option<String>,
    pub final_video_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StyledSlide {
    pub index: u32,
    pub original_url: String,
    pub styled_url: String,
    pub timestamp: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CleanedSlide {
    pub index: u32,
    pub original_url: String,
    pub cleaned_url: String,
    pub timestamp: f64,
}
