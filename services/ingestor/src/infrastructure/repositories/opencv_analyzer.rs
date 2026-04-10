use async_trait::async_trait;
use anyhow::Result;
use std::path::PathBuf;
use opencv::{
    prelude::*,
    videoio::{VideoCapture, CAP_ANY, CAP_PROP_FRAME_COUNT, CAP_PROP_FPS, CAP_PROP_POS_MSEC},
    imgcodecs,
    imgproc,
    core::{self, absdiff, sum_elems, Mat},
};
use keryx_core::domain::ports::video_repository::VideoAnalyzer;

pub struct OpenCvAnalyzer {
    output_dir: PathBuf,
    threshold: f64,
}

impl OpenCvAnalyzer {
    pub fn new(output_dir: PathBuf, threshold: f64) -> Self {
        Self { output_dir, threshold }
    }
}

#[async_trait]
impl VideoAnalyzer for OpenCvAnalyzer {
    async fn detect_slides(&self, video_path: &PathBuf) -> Result<Vec<(u32, f64, PathBuf)>> {
        let mut cap = VideoCapture::from_file(video_path.to_str().unwrap(), CAP_ANY)?;
        if !cap.is_opened()? {
            return Err(anyhow::anyhow!("Could not open video file"));
        }

        let total_frames = cap.get(CAP_PROP_FRAME_COUNT)? as i32;
        let _fps = cap.get(CAP_PROP_FPS)?;
        let mut slides = Vec::new();
        let mut prev_frame = Mat::default();
        let mut slide_index = 0;

        for i in 0..total_frames {
            let mut frame = Mat::default();
            if !cap.read(&mut frame)? {
                break;
            }

            let mut gray = Mat::default();
            imgproc::cvt_color(&frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;

            if i == 0 {
                // Save first slide
                let timestamp = cap.get(CAP_PROP_POS_MSEC)? / 1000.0;
                let frame_path = self.output_dir.join(format!("frame_{}.png", slide_index));
                imgcodecs::imwrite(&frame_path.to_str().unwrap(), &frame, &core::Vector::new())?;
                slides.push((slide_index, timestamp, frame_path));
                slide_index += 1;
                prev_frame = gray;
                continue;
            }

            let mut diff = Mat::default();
            absdiff(&gray, &prev_frame, &mut diff)?;
            let diff_sum: f64 = sum_elems(&diff)?.0[0];
            let mean_diff = diff_sum / (gray.rows() * gray.cols()) as f64;

            if mean_diff > self.threshold {
                let timestamp = cap.get(CAP_PROP_POS_MSEC)? / 1000.0;
                let frame_path = self.output_dir.join(format!("frame_{}.png", slide_index));
                imgcodecs::imwrite(&frame_path.to_str().unwrap(), &frame, &core::Vector::new())?;
                slides.push((slide_index, timestamp, frame_path));
                slide_index += 1;
            }

            prev_frame = gray;
        }

        Ok(slides)
    }
}
