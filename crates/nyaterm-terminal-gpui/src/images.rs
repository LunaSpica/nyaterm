//! Decode terminal graphics payloads into GPUI `RenderImage`s.
use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use gpui::RenderImage;
use image::{Frame, ImageReader, Limits};
use smallvec::SmallVec;

/// Process-wide decode cache keyed by placement id + payload fingerprint.
static DECODE_CACHE: Mutex<Option<DecodeCache>> = Mutex::new(None);

const MAX_DECODE_CACHE_IMAGES: usize = 128;
const MAX_DECODED_IMAGE_DIMENSION: u32 = 4096;
const MAX_DECODED_IMAGE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Default)]
struct DecodeCache {
    entries: HashMap<u64, Arc<RenderImage>>,
    lru: VecDeque<u64>,
}

impl DecodeCache {
    fn get(&mut self, key: u64) -> Option<Arc<RenderImage>> {
        let image = self.entries.get(&key)?.clone();
        self.touch(key);
        Some(image)
    }

    fn insert(&mut self, key: u64, image: Arc<RenderImage>) {
        self.entries.insert(key, image);
        self.touch(key);
        while self.entries.len() > MAX_DECODE_CACHE_IMAGES {
            let Some(oldest) = self.lru.pop_front() else {
                break;
            };
            if self.entries.remove(&oldest).is_some() {
                self.lru.retain(|candidate| *candidate != oldest);
            }
        }
    }

    fn touch(&mut self, key: u64) {
        self.lru.retain(|candidate| *candidate != key);
        self.lru.push_back(key);
    }
}

fn cache() -> std::sync::MutexGuard<'static, Option<DecodeCache>> {
    DECODE_CACHE.lock().unwrap_or_else(|e| e.into_inner())
}

fn fingerprint(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

fn cache_key(placement_id: u64, data: &[u8]) -> u64 {
    placement_id ^ fingerprint(data).rotate_left(17)
}

/// Decode encoded image bytes (NYAR RGBA / PNG/JPEG/GIF/BMP) into a BGRA `RenderImage`.
pub fn decode_render_image(data: &[u8]) -> Option<Arc<RenderImage>> {
    if data.is_empty() {
        return None;
    }
    let mut rgba = if let Some((w, h, raw)) = unpack_nyar(data) {
        image::RgbaImage::from_raw(w, h, raw)?
    } else {
        decode_compressed_image(data)?
    };
    // GPUI atlas expects BGRA.
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let frames = SmallVec::from_elem(Frame::new(rgba), 1);
    Some(Arc::new(RenderImage::new(frames)))
}

fn decode_compressed_image(data: &[u8]) -> Option<image::RgbaImage> {
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .ok()?;
    reader.limits(decode_limits());
    let image = reader.decode().ok()?;
    let width = image.width();
    let height = image.height();
    decoded_rgba_bytes(width, height)?;
    Some(image.into_rgba8())
}

fn decode_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DECODED_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_DECODED_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODED_IMAGE_BYTES);
    limits
}

fn decoded_rgba_bytes(width: u32, height: u32) -> Option<u64> {
    if width == 0
        || height == 0
        || width > MAX_DECODED_IMAGE_DIMENSION
        || height > MAX_DECODED_IMAGE_DIMENSION
    {
        return None;
    }
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(4)?;
    (bytes <= MAX_DECODED_IMAGE_BYTES).then_some(bytes)
}

/// NyaTerm intermediate raster container produced by the Sixel decoder:
/// `NYAR` + width:u32le + height:u32le + RGBA8 pixels.
fn unpack_nyar(data: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if data.len() < 12 || &data[..4] != b"NYAR" {
        return None;
    }
    let width = u32::from_le_bytes(data[4..8].try_into().ok()?);
    let height = u32::from_le_bytes(data[8..12].try_into().ok()?);
    let need = usize::try_from(decoded_rgba_bytes(width, height)?).ok()?;
    if data.len() < 12 + need {
        return None;
    }
    Some((width, height, data[12..12 + need].to_vec()))
}

/// Cached decode for a placement. Returns `None` when payload is not a raster image.
pub fn cached_render_image(placement_id: u64, data: &[u8]) -> Option<Arc<RenderImage>> {
    if data.is_empty() {
        return None;
    }
    let key = cache_key(placement_id, data);
    {
        let mut guard = cache();
        let cache = guard.get_or_insert_with(DecodeCache::default);
        if let Some(hit) = cache.get(key) {
            return Some(hit);
        }
    }
    let decoded = decode_render_image(data)?;
    let mut guard = cache();
    let cache = guard.get_or_insert_with(DecodeCache::default);
    cache.insert(key, decoded.clone());
    Some(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_cache() {
        *cache() = None;
    }

    fn tiny_nyar(pixel: [u8; 4]) -> Vec<u8> {
        let mut data = b"NYAR".to_vec();
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&pixel);
        data
    }

    fn tiny_png() -> Vec<u8> {
        // 1x1 red PNG
        let rgba = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
        png_from_rgba(rgba)
    }

    fn png_from_rgba(rgba: image::RgbaImage) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut bytes);
        image::DynamicImage::ImageRgba8(rgba)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .expect("encode png");
        bytes
    }

    #[test]
    fn decodes_png_to_render_image() {
        let png = tiny_png();
        let image = decode_render_image(&png).expect("decode");
        assert_eq!(image.frame_count(), 1);
        let size = image.size(0);
        assert_eq!(u32::from(size.width), 1);
        assert_eq!(u32::from(size.height), 1);
    }

    #[test]
    fn rejects_png_that_expands_past_decode_budget() {
        let width = 1025;
        let height = 1025;
        assert!(decoded_rgba_bytes(width, height).is_none());

        let rgba = image::RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 0]));
        let png = png_from_rgba(rgba);
        assert!(png.len() as u64 <= MAX_DECODED_IMAGE_BYTES);
        assert!(decode_render_image(&png).is_none());
    }

    #[test]
    fn rejects_nyar_that_expands_past_decode_budget() {
        let mut data = b"NYAR".to_vec();
        data.extend_from_slice(&1025u32.to_le_bytes());
        data.extend_from_slice(&1025u32.to_le_bytes());
        assert!(decode_render_image(&data).is_none());
    }

    #[test]
    fn cache_returns_same_arc_for_same_payload() {
        clear_cache();
        let png = tiny_png();
        let a = cached_render_image(42, &png).expect("a");
        let b = cached_render_image(42, &png).expect("b");
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn cache_prunes_least_recently_used_image() {
        clear_cache();
        let first = tiny_nyar([0, 0, 0, 255]);
        let first_image = cached_render_image(1, &first).expect("first");
        for placement_id in 2..=MAX_DECODE_CACHE_IMAGES as u64 {
            let value = placement_id as u8;
            let payload = tiny_nyar([value, 0, 0, 255]);
            cached_render_image(placement_id, &payload).expect("fill");
        }
        assert!(Arc::ptr_eq(
            &first_image,
            &cached_render_image(1, &first).expect("first still cached")
        ));

        let second = tiny_nyar([2, 0, 0, 255]);
        let second_image = cached_render_image(2, &second).expect("second before eviction");
        let overflow = tiny_nyar([255, 0, 0, 255]);
        cached_render_image(999, &overflow).expect("overflow");

        assert!(Arc::ptr_eq(
            &first_image,
            &cached_render_image(1, &first).expect("recent first kept")
        ));
        assert!(!Arc::ptr_eq(
            &second_image,
            &cached_render_image(2, &second).expect("old second decoded again")
        ));
    }

    #[test]
    fn cache_key_uses_entire_payload() {
        let mut a = vec![0u8; 192];
        let mut b = vec![0u8; 192];
        a[96] = 1;
        b[96] = 2;

        assert_ne!(cache_key(42, &a), cache_key(42, &b));
    }

    #[test]
    fn decodes_nyar_rgba_from_sixel_path() {
        // 1x1 blue pixel packed as NYAR.
        let data = tiny_nyar([0, 0, 255, 255]);
        let image = decode_render_image(&data).expect("decode nyar");
        assert_eq!(image.frame_count(), 1);
        let size = image.size(0);
        assert_eq!(u32::from(size.width), 1);
        assert_eq!(u32::from(size.height), 1);
    }
}
