use std::{fs, io::{Read, Seek, SeekFrom}, path::Path};

use image::{AnimationDecoder, RgbaImage};

use super::AnimInfo;

const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

#[derive(Default)]
struct Probe {
  width: u32,
  height: u32,
  frame_count: Option<u32>,
  delay_frames: u32,
  duration_ms: u64,
}

impl Probe {
  fn apply(&mut self, kind: &[u8], chunk: &[u8], position: u64) -> Option<()> {
    match kind {
      b"IHDR" if position == 8 && chunk.len() >= 8 => {
        self.width = u32::from_be_bytes(chunk[..4].try_into().ok()?);
        self.height = u32::from_be_bytes(chunk[4..8].try_into().ok()?);
      }
      b"acTL" if chunk.len() >= 8 => {
        self.frame_count = Some(u32::from_be_bytes(chunk[..4].try_into().ok()?));
      }
      b"fcTL" if chunk.len() >= 26 => {
        self.delay_frames += 1;
        let numerator = u64::from(u16::from_be_bytes([chunk[20], chunk[21]]));
        let denominator = u64::from(u16::from_be_bytes([chunk[22], chunk[23]]));
        self.duration_ms += numerator * 1000 / if denominator == 0 { 100 } else { denominator };
      }
      _ => {}
    }
    Some(())
  }

  fn finish(self) -> Option<AnimInfo> {
    let frame_count = self.frame_count.filter(|count| *count > 1 && *count == self.delay_frames)?;
    if self.width == 0 || self.height == 0 {
      return None;
    }
    Some(AnimInfo { width: self.width, height: self.height, frame_count, duration_secs: self.duration_ms as f64 / 1000.0 })
  }
}

pub fn probe(path: &Path) -> Option<AnimInfo> {
  let mut file = fs::File::open(path).ok()?;
  let mut signature = [0; 8];
  file.read_exact(&mut signature).ok()?;
  if signature != SIGNATURE {
    return None;
  }
  let mut probe = Probe::default();
  let mut position = 8_u64;
  loop {
    let mut header = [0; 8];
    file.read_exact(&mut header).ok()?;
    let len = u32::from_be_bytes(header[..4].try_into().ok()?) as usize;
    let kind = &header[4..8];
    if matches!(kind, b"IHDR" | b"acTL" | b"fcTL") {
      if len > 26 {
        return None;
      }
      let mut chunk = vec![0; len];
      file.read_exact(&mut chunk).ok()?;
      let mut crc = [0; 4];
      file.read_exact(&mut crc).ok()?;
      probe.apply(kind, &chunk, position)?;
    } else {
      file.seek(SeekFrom::Current(len as i64 + 4)).ok()?;
    }
    position += len as u64 + 12;
    if kind == b"IEND" {
      break;
    }
  }
  probe.finish()
}

#[cfg(test)]
fn parse(data: &[u8]) -> Option<AnimInfo> {
  if data.len() < 8 || data[..8] != SIGNATURE {
    return None;
  }
  let mut probe = Probe::default();
  let mut position = 8_usize;
  while position + 12 <= data.len() {
    let len = u32::from_be_bytes(data[position..position + 4].try_into().ok()?) as usize;
    let end = position.checked_add(len + 12)?;
    if end > data.len() {
      return None;
    }
    probe.apply(&data[position + 4..position + 8], &data[position + 8..position + 8 + len], position as u64)?;
    if &data[position + 4..position + 8] == b"IEND" {
      break;
    }
    position = end;
  }
  probe.finish()
}

pub fn stream(path: &Path, on_frame: &mut impl FnMut(u32, RgbaImage, u32) -> bool) -> Option<()> {
  let decoder = image::codecs::png::PngDecoder::new(std::io::BufReader::new(fs::File::open(path).ok()?)).ok()?;
  if !decoder.is_apng().ok()? {
    return None;
  }
  for (index, frame) in decoder.apng().ok()?.into_frames().enumerate() {
    let index = index as u32;
    let frame = match frame {
      Ok(frame) => frame,
      Err(error) => {
        log::warn!("APNG 第 {index} 帧解码失败，跳过（{}）：{error}", path.display());
        continue;
      }
    };
    let (numerator, denominator) = frame.delay().numer_denom_ms();
    if !on_frame(index, frame.into_buffer(), numerator / denominator.max(1)) {
      break;
    }
  }
  Some(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn apng_delay_zero_denominator_means_one_hundred() {
    let mut data = SIGNATURE.to_vec();
    append(&mut data, *b"IHDR", &[0, 0, 0, 2, 0, 0, 0, 3]);
    append(&mut data, *b"acTL", &[0, 0, 0, 2, 0, 0, 0, 0]);
    let mut frame = vec![0; 26];
    frame[21] = 1;
    append(&mut data, *b"fcTL", &frame);
    append(&mut data, *b"fcTL", &frame);
    let info = parse(&data).unwrap();
    assert_eq!((info.width, info.height, info.frame_count), (2, 3, 2));
    assert_eq!(info.duration_secs, 0.02);
  }

  #[test]
  #[ignore]
  fn real_apng_probe_and_decode() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets/anim3.png");
    let info = probe(&path).expect("真实 APNG 应可探测");
    let mut delays = Vec::new();
    stream(&path, &mut |_, _, delay| {
      delays.push(delay);
      true
    }).expect("真实 APNG 应可解码");
    assert_eq!(delays.len(), info.frame_count as usize);
    assert_eq!(delays.iter().map(|delay| u64::from(*delay)).sum::<u64>(), (info.duration_secs * 1000.0) as u64);
  }

  fn append(data: &mut Vec<u8>, kind: [u8; 4], payload: &[u8]) {
    data.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    data.extend_from_slice(&kind);
    data.extend_from_slice(payload);
    data.extend_from_slice(&[0; 4]);
  }
}
