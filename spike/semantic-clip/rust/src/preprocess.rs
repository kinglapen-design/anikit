use std::{path::Path, time::Instant};

use anyhow::{Context, Result, bail};
use image::imageops::FilterType;
use serde::Serialize;
use sha2::{Digest, Sha256};

const WIDTH: usize = 224;
const HEIGHT: usize = 224;
const MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
const STD: [f32; 3] = [0.26862954, 0.26130258, 0.27577711];

pub struct NativeImage {
    pub data: Vec<f32>,
    pub source_width: u32,
    pub source_height: u32,
    pub elapsed_ms: f64,
}

#[derive(Serialize)]
pub struct PreprocessComparison {
    pub elements: usize,
    pub max_abs_diff: f32,
    pub mean_abs_diff: f64,
    pub rms_diff: f64,
    pub exact_element_fraction: f64,
}

pub fn load_jpeg(path: &Path) -> Result<NativeImage> {
    let start = Instant::now();
    let image = image::open(path).with_context(|| format!("decode JPEG {}", path.display()))?;
    let (source_width, source_height) = (image.width(), image.height());
    let rgb = image
        .resize_exact(WIDTH as u32, HEIGHT as u32, FilterType::CatmullRom)
        .to_rgb8();
    let mut data = vec![0.0_f32; 3 * WIDTH * HEIGHT];
    for (x, y, pixel) in rgb.enumerate_pixels() {
        let offset = y as usize * WIDTH + x as usize;
        for channel in 0..3 {
            let value = f32::from(pixel[channel]) / 255.0;
            data[channel * WIDTH * HEIGHT + offset] = (value - MEAN[channel]) / STD[channel];
        }
    }
    if data.iter().any(|value| !value.is_finite()) {
        bail!("native preprocessing produced NaN or infinity");
    }
    Ok(NativeImage {
        data,
        source_width,
        source_height,
        elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

pub fn compare(native: &[f32], python: &[f32]) -> Result<PreprocessComparison> {
    if native.len() != 3 * WIDTH * HEIGHT || native.len() != python.len() {
        bail!(
            "preprocessing tensor lengths differ: native={}, python={}",
            native.len(),
            python.len()
        );
    }
    let mut max_abs_diff = 0.0_f32;
    let mut sum_abs_diff = 0.0_f64;
    let mut sum_squared_diff = 0.0_f64;
    let mut exact = 0_usize;
    for (left, right) in native.iter().zip(python) {
        let difference = (left - right).abs();
        max_abs_diff = max_abs_diff.max(difference);
        sum_abs_diff += f64::from(difference);
        sum_squared_diff += f64::from(difference * difference);
        if left.to_bits() == right.to_bits() {
            exact += 1;
        }
    }
    let elements = native.len();
    Ok(PreprocessComparison {
        elements,
        max_abs_diff,
        mean_abs_diff: sum_abs_diff / elements as f64,
        rms_diff: (sum_squared_diff / elements as f64).sqrt(),
        exact_element_fraction: exact as f64 / elements as f64,
    })
}

pub fn little_endian_bytes(data: &[f32]) -> Vec<u8> {
    data.iter().flat_map(|value| value.to_le_bytes()).collect()
}

pub fn sha256(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
