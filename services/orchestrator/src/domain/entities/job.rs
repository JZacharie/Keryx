use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    pub id: String,
    pub status: JobStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum JobStatus {
    Ingested,
    Downloading,
    Transcribing,
    Analyzing,
    Translating,
    GeneratingVisuals,
    CloningVoice,
    Composing,
    Completed,
    Failed(String),
}
