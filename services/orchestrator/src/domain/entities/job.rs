use serde::{Serialize, Deserialize};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    pub id: String,
    pub status: JobStatus,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum JobStatus {
    Ingested,
    Processing,
    Completed,
    Failed,
}
