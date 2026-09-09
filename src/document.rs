use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    path::Path,
};

pub type Pixel = [u8; 4];
pub const CLEAR: Pixel = [0; 4];
const MAX_PIXELS: usize = 16_777_216;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: u8,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Frame {
    pub duration: u32,
    pub cels: Vec<Vec<Pixel>>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Document {
    pub version: u32,
    pub width: usize,
    pub height: usize,
    pub layers: Vec<Layer>,
    pub frames: Vec<Frame>,
    pub palette: Vec<Pixel>,
}
impl Document {
    pub fn new(width: usize, height: usize) -> Self {
        let palette = [
            0x171c2c, 0x303953, 0x59647a, 0x8996a5, 0xc5cbd0, 0xf4f1de, 0x583f6e, 0x9364aa,
            0xcc8fcb, 0x8c3f5d, 0xcf6679, 0xf2a49e, 0x985439, 0xd58b47, 0xf4ce75, 0x475b45,
            0x718f58, 0xa5c778, 0x245964, 0x388b91, 0x80c5bb, 0x304b80, 0x547fc0, 0x91b8e3,
        ]
        .map(|c| [(c >> 16) as u8, (c >> 8) as u8, c as u8, 255])
        .to_vec();
        Self {
            version: 1,
            width,
            height,
            layers: vec![Layer {
                name: "Layer 1".into(),
                visible: true,
                locked: false,
                opacity: 255,
            }],
            frames: vec![Frame {
                duration: 100,
                cels: vec![vec![CLEAR; width * height]],
            }],
            palette,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let n = self
            .width
            .checked_mul(self.height)
            .ok_or("Canvas too large")?;
        if self.version != 1
            || self.width == 0
            || self.height == 0
            || self.width > 2048
            || self.height > 2048
            || self.layers.is_empty()
            || self.frames.is_empty()
            || self.layers.len() > 128
            || self.frames.len() > 1024
            || self.palette.len() > 256
        {
            return Err("Unsupported or invalid project dimensions".into());
        }
        if n.saturating_mul(self.layers.len())
            .saturating_mul(self.frames.len())
            > MAX_PIXELS
        {
            return Err("Project exceeds 16 million stored pixels".into());
        }
        for frame in &self.frames {
            if !(10..=10000).contains(&frame.duration)
                || frame.cels.len() != self.layers.len()
                || frame.cels.iter().any(|c| c.len() != n)
            {
                return Err("Invalid frame data".into());
            }
        }
        Ok(())
    }
    pub fn can_grow(&self, layers: usize, frames: usize) -> bool {
        layers <= 128 && frames <= 1024 && self.width * self.height * layers * frames <= MAX_PIXELS
    }
    pub fn check_size(&self, width: usize, height: usize) -> Result<(), String> {
        if !(1..=2048).contains(&width) || !(1..=2048).contains(&height) {
            return Err("Width and height must be between 1 and 2048 pixels.".into());
        }
        if width
            .saturating_mul(height)
            .saturating_mul(self.layers.len())
            .saturating_mul(self.frames.len())
            > MAX_PIXELS
        {
            return Err(
                "This size exceeds the 16 million stored pixel limit across all layers and frames."
                    .into(),
            );
        }
        Ok(())
    }
    /// Resize all cels, including locked/hidden layers. None scales artwork;
    /// Some(offset) translates existing pixels into an expanded/cropped canvas.
    pub fn resized(
        &self,
        width: usize,
        height: usize,
        offset: Option<(i32, i32)>,
    ) -> Result<Self, String> {
        self.validate()?;
        self.check_size(width, height)?;
        let mut result = Self {
            width,
            height,
            frames: Vec::with_capacity(self.frames.len()),
            version: self.version,
            layers: self.layers.clone(),
            palette: self.palette.clone(),
        };
        for frame in &self.frames {
            let mut cels = Vec::with_capacity(frame.cels.len());
            for cel in &frame.cels {
                let mut pixels = vec![CLEAR; width * height];
                for y in 0..height {
                    for x in 0..width {
                        let (sx, sy) = match offset {
                            Some((dx, dy)) => (x as i64 - dx as i64, y as i64 - dy as i64),
                            None => (
                                (x * self.width / width) as i64,
                                (y * self.height / height) as i64,
                            ),
                        };
                        if sx >= 0 && sy >= 0 && sx < self.width as i64 && sy < self.height as i64 {
                            pixels[y * width + x] = cel[sy as usize * self.width + sx as usize];
                        }
                    }
                }
                cels.push(pixels);
            }
            result.frames.push(Frame {
                duration: frame.duration,
                cels,
            });
        }
        Ok(result)
    }
    pub fn composite(&self, frame: usize) -> Vec<Pixel> {
        let mut out = vec![CLEAR; self.width * self.height];
        for (layer, cel) in self.layers.iter().zip(&self.frames[frame].cels) {
            if !layer.visible {
                continue;
            }
            for (dst, src) in out.iter_mut().zip(cel) {
                *dst = over(*dst, *src, layer.opacity);
            }
        }
        out
    }
    pub fn put(&mut self, frame: usize, layer: usize, x: i32, y: i32, color: Pixel) {
        if x >= 0
            && y >= 0
            && x < (self.width as i32)
            && y < (self.height as i32)
            && !self.layers[layer].locked
        {
            self.frames[frame].cels[layer][y as usize * self.width + x as usize] = color;
        }
    }
    pub fn fill(&mut self, frame: usize, layer: usize, x: i32, y: i32, color: Pixel) {
        if x < 0
            || y < 0
            || x >= self.width as i32
            || y >= self.height as i32
            || self.layers[layer].locked
        {
            return;
        }
        let target = self.frames[frame].cels[layer][y as usize * self.width + x as usize];
        if target == color {
            return;
        }
        let mut queue = VecDeque::from([(x, y)]);
        while let Some((x, y)) = queue.pop_front() {
            if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                continue;
            }
            let i = y as usize * self.width + x as usize;
            if self.frames[frame].cels[layer][i] != target {
                continue;
            }
            self.frames[frame].cels[layer][i] = color;
            queue.extend([(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]);
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        serde_json::to_writer(temp.as_file_mut(), self).map_err(|e| e.to_string())?;
        temp.flush().map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(160_000_001)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 160_000_000 {
            return Err("Project file exceeds 160 MB".into());
        }
        let doc: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        doc.validate()?;
        Ok(doc)
    }
}
pub fn over(dst: Pixel, src: Pixel, opacity: u8) -> Pixel {
    let a = src[3] as f32 / 255.0 * opacity as f32 / 255.0;
    let b = dst[3] as f32 / 255.0 * (1.0 - a);
    let total = a + b;
    if total == 0.0 {
        return CLEAR;
    }
    let mut out = [0; 4];
    for i in 0..3 {
        out[i] = ((src[i] as f32 * a + dst[i] as f32 * b) / total).round() as u8;
    }
    out[3] = (total * 255.0).round() as u8;
    out
}
pub fn line(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = a;
    let dx = (b.0 - x).abs();
    let sx = if x < b.0 { 1 } else { -1 };
    let dy = -(b.1 - y).abs();
    let sy = if y < b.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut points = Vec::new();
    loop {
        points.push((x, y));
        if (x, y) == b {
            break;
        }
        let e = 2 * err;
        if e >= dy {
            err += dy;
            x += sx;
        }
        if e <= dx {
            err += dx;
            y += sy;
        }
    }
    points
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canvas_resize_translates_crops_and_preserves_metadata() {
        let mut d = Document::new(2, 2);
        d.frames[0].cels[0] = vec![[1; 4], [2; 4], [3; 4], [4; 4]];
        d.frames.push(d.frames[0].clone());
        d.frames[1].duration = 250;
        d.layers[0].locked = true;
        d.layers[0].visible = false;
        let expanded = d.resized(4, 4, Some((1, 1))).unwrap();
        assert_eq!(expanded.frames[0].cels[0][5], [1; 4]);
        assert_eq!(expanded.frames[1].cels[0][10], [4; 4]);
        assert_eq!(expanded.frames[0].cels[0][0], CLEAR);
        assert_eq!(expanded.layers, d.layers);
        assert_eq!(expanded.frames[1].duration, 250);
        assert_eq!(expanded.resized(2, 2, Some((-1, -1))).unwrap(), d);
        let cropped = d.resized(1, 1, Some((-1, -1))).unwrap();
        assert_eq!(cropped.frames[0].cels[0], vec![[4; 4]]);
    }
    #[test]
    fn sprite_resize_scales_exact_rgba_in_all_cels() {
        let mut d = Document::new(2, 1);
        d.frames[0].cels[0] = vec![[10, 20, 30, 128], [40, 50, 60, 255]];
        d.layers.push(d.layers[0].clone());
        d.frames[0].cels.push(vec![[7; 4], CLEAR]);
        d.frames.push(d.frames[0].clone());
        let scaled = d.resized(4, 2, None).unwrap();
        assert_eq!(
            scaled.frames[0].cels[0],
            vec![
                [10, 20, 30, 128],
                [10, 20, 30, 128],
                [40, 50, 60, 255],
                [40, 50, 60, 255],
                [10, 20, 30, 128],
                [10, 20, 30, 128],
                [40, 50, 60, 255],
                [40, 50, 60, 255]
            ]
        );
        assert_eq!(
            scaled.frames[1].cels[1],
            vec![[7; 4], [7; 4], CLEAR, CLEAR, [7; 4], [7; 4], CLEAR, CLEAR]
        );
        assert_eq!(scaled.resized(2, 1, None).unwrap(), d);
        assert!(d.resized(0, 1, None).is_err());
        assert!(d.resized(usize::MAX, 1, None).is_err());
    }
    #[test]
    fn flood_respects_boundaries() {
        let mut d = Document::new(5, 5);
        for y in 0..5 {
            d.put(0, 0, 2, y, [255; 4]);
        }
        d.fill(0, 0, 0, 0, [80; 4]);
        assert_eq!(d.frames[0].cels[0][0], [80; 4]);
        assert_eq!(d.frames[0].cels[0][4], CLEAR);
    }
    #[test]
    fn alpha_composites_without_dark_fringe() {
        assert_eq!(over(CLEAR, [255, 0, 0, 128], 255), [255, 0, 0, 128]);
        assert_eq!(
            over([0, 0, 255, 255], [255, 0, 0, 128], 255),
            [128, 0, 127, 255]
        );
    }
    #[test]
    fn locked_layer_is_immutable() {
        let mut d = Document::new(2, 2);
        d.layers[0].locked = true;
        d.fill(0, 0, 0, 0, [255; 4]);
        d.put(0, 0, 1, 1, [255; 4]);
        assert!(d.frames[0].cels[0].iter().all(|p| *p == CLEAR));
    }
    #[test]
    fn lines_include_endpoints_in_all_octants() {
        for b in [(-4, -2), (4, -2), (-2, 4), (2, 4), (0, 0)] {
            let l = line((0, 0), b);
            assert_eq!(l.first(), Some(&(0, 0)));
            assert_eq!(l.last(), Some(&b));
            assert!(
                l.windows(2)
                    .all(|w| (w[0].0 - w[1].0).abs() <= 1 && (w[0].1 - w[1].1).abs() <= 1)
            );
        }
    }
    #[test]
    fn project_roundtrip_and_validation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.mote");
        let mut d = Document::new(4, 4);
        d.put(0, 0, 1, 2, [1, 2, 3, 255]);
        d.save(&path).unwrap();
        assert_eq!(Document::load(&path).unwrap(), d);
        d.frames[0].cels[0].pop();
        assert!(d.validate().is_err());
    }
}
