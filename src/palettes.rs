use mote::document::{Document, Pixel};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    pub colors: Vec<Pixel>,
}
fn preset(name: &str, hex: &str) -> Palette {
    Palette {
        name: name.into(),
        colors: hex
            .split_whitespace()
            .map(|s| {
                let c = u32::from_str_radix(s, 16).expect("built-in palette");
                [(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]
            })
            .collect(),
    }
}
pub fn presets() -> Vec<Palette> {
    vec![
        Palette {
            name: "Mote · 24 colors".into(),
            colors: Document::new(1, 1).palette,
        },
        preset(
            "Sweetie 16 · GrafxKid",
            "1a1c2c 5d275d b13e53 ef7d57 ffcd75 a7f070 38b764 257179 29366f 3b5dc9 41a6f6 73eff7 f4f4f4 94b0c2 566c86 333c57",
        ),
        preset(
            "ENDESGA 32 · ENDESGA",
            "be4a2f d77643 ead4aa e4a672 b86f50 733e39 3e2731 a22633 e43b44 f77622 feae34 fee761 63c74d 3e8948 265c42 193c3e 124e89 0099db 2ce8f5 ffffff c0cbdc 8b9bb4 5a6988 3a4466 262b44 181425 ff0044 68386c b55088 f6757a e8b796 c28569",
        ),
        preset(
            "Google-inspired · 8 colors",
            "4285f4 ea4335 fbbc05 34a853 202124 5f6368 e8eaed ffffff",
        ),
        preset("Retro handheld · 4 colors", "0f380f 306230 8bac0f 9bbc0f"),
        Palette {
            name: "Grayscale · 16 colors".into(),
            colors: (0..16).map(|n| [n * 17, n * 17, n * 17, 255]).collect(),
        },
    ]
}
pub fn library_path() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        })
        .join("mote/palettes.json")
}
pub fn validate(library: &[Palette]) -> Result<(), String> {
    if library.len() > 256
        || library.iter().any(|p| {
            p.name.trim().is_empty()
                || p.name.len() > 100
                || p.colors.is_empty()
                || p.colors.len() > 256
        })
    {
        return Err(
            "Use a name of 1–100 bytes and 1–256 colors (up to 256 saved palettes).".into(),
        );
    }
    Ok(())
}
pub fn load() -> Result<Vec<Palette>, String> {
    let path = library_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(2_000_001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 2_000_000 {
        return Err("Palette library exceeds 2 MB".into());
    }
    let library: Vec<Palette> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    validate(&library)?;
    Ok(library)
}
pub fn save(library: &[Palette]) -> Result<(), String> {
    validate(library)?;
    let path = library_path();
    let parent = path.parent().unwrap();
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer(temp.as_file_mut(), library).map_err(|e| e.to_string())?;
    temp.flush().map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builtins_have_expected_colors_and_validate() {
        let p = presets();
        validate(&p).unwrap();
        assert_eq!(p[1].colors.len(), 16);
        assert_eq!(p[2].colors.len(), 32);
        assert_eq!(p[1].colors[0], [26, 28, 44, 255]);
        assert!(p.iter().flat_map(|p| &p.colors).all(|c| c[3] == 255));
    }
    #[test]
    fn library_rejects_oversized_and_empty_palettes() {
        assert!(
            validate(&[Palette {
                name: "Empty".into(),
                colors: vec![]
            }])
            .is_err()
        );
        assert!(
            validate(&[Palette {
                name: "Too large".into(),
                colors: vec![[0; 4]; 257]
            }])
            .is_err()
        );
        let p = Palette {
            name: "Translucent".into(),
            colors: vec![[10, 20, 30, 128]],
        };
        let encoded = serde_json::to_string(&vec![p]).unwrap();
        let decoded: Vec<Palette> = serde_json::from_str(&encoded).unwrap();
        validate(&decoded).unwrap();
        assert_eq!(decoded[0].colors[0][3], 128);
    }
}
