#[allow(dead_code)]
pub struct Job {
    pub id: String,
    pub status: JobStatus,
}

#[allow(dead_code)]
pub enum JobStatus {
    Ingested,
    Processing,
    Completed,
    Failed,
}
