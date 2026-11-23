//! Artwork → resized lossy WebP (alpha kept); small pixel sprites are copied as PNG.

use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Img {
    /// path inside the dictionary zip
    pub zip_path: String,
    pub file: PathBuf,
    pub pixelated: bool,
}

pub fn pokemon(cache: &Path, build: &Path, ids: &[u32], size: u32, quality: f32) -> HashMap<u32, Img> {
    let out_dir = build.join("img/p");
    fs::create_dir_all(&out_dir).unwrap();
    ids.par_iter()
        .filter_map(|&id| {
            let src = cache.join(format!("sprites/pokemon/{id}.png"));
            if !src.exists() {
                return None;
            }
            let img = image::open(&src).unwrap_or_else(|e| panic!("{}: {e}", src.display()));
            if img.width() <= 128 {
                // classic 96px pixel sprite fallback: keep as-is
                let dst = out_dir.join(format!("{id}.png"));
                fs::copy(&src, &dst).unwrap();
                return Some((id, Img { zip_path: format!("img/p/{id}.png"), file: dst, pixelated: true }));
            }
            let dst = out_dir.join(format!("{id}.webp"));
            if !dst.exists() {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let (nw, nh) = if w >= h { (size, (h * size / w).max(1)) } else { ((w * size / h).max(1), size) };
                let small = image::imageops::resize(&rgba, nw, nh, image::imageops::FilterType::Lanczos3);
                let enc = webp::Encoder::from_rgba(small.as_raw(), nw, nh).encode(quality);
                fs::write(&dst, &*enc).unwrap();
            }
            Some((id, Img { zip_path: format!("img/p/{id}.webp"), file: dst, pixelated: false }))
        })
        .collect()
}

/// Item sprites: ≤32px pixel art copied as PNG; larger (64–160px) icons resized to 96px WebP.
/// Byte-identical sources (generic category icons shared by hundreds of items) map to one file.
pub fn items(cache: &Path, build: &Path, idents: &[String], quality: f32) -> HashMap<String, Img> {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let out_dir = build.join("img/i");
    fs::create_dir_all(&out_dir).unwrap();
    let converted: Vec<(String, u64, Img)> = idents
        .par_iter()
        .filter_map(|ident| {
            let src = cache.join(format!("sprites/items/{ident}.png"));
            let bytes = fs::read(&src).ok()?;
            let mut h = DefaultHasher::new();
            bytes.hash(&mut h);
            let key = h.finish();
            let img = image::load_from_memory(&bytes).ok()?;
            if img.width() <= 32 {
                return Some((ident.clone(), key, Img { zip_path: format!("img/i/{key:016x}.png"), file: src, pixelated: true }));
            }
            let dst = out_dir.join(format!("{key:016x}.webp"));
            if !dst.exists() {
                let size = 96.min(img.width());
                let small = image::imageops::resize(&img.to_rgba8(), size, size, image::imageops::FilterType::Lanczos3);
                let enc = webp::Encoder::from_rgba(small.as_raw(), size, size).encode(quality);
                fs::write(&dst, &*enc).unwrap();
            }
            Some((ident.clone(), key, Img { zip_path: format!("img/i/{key:016x}.webp"), file: dst, pixelated: false }))
        })
        .collect();
    converted.into_iter().map(|(ident, _, img)| (ident, img)).collect()
}
