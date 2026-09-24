use crate::backend::core::http::client;
use crate::backend::core::tasks::spawn_io;
use crate::config::Config;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex, RwLock};

#[derive(Clone, Debug)]
pub struct DecodedImage {
    pub width: i32,
    pub height: i32,
    pub rgba: Vec<u8>,
    pub average_color: Option<(u8, u8, u8)>,
}

impl DecodedImage {
    /// Converts the decoded RGBA buffer into a GDK texture on the current thread.
    pub fn to_texture(&self) -> gtk::gdk::Texture {
        let gbytes = gtk::glib::Bytes::from(&self.rgba);
        let mem_tex = gtk::gdk::MemoryTexture::new(
            self.width,
            self.height,
            gtk::gdk::MemoryFormat::R8g8b8a8,
            &gbytes,
            (self.width * 4) as usize,
        );
        use adw::prelude::Cast;
        mem_tex.upcast()
    }
}

struct ImageCacheState {
    memory_cache: RwLock<HashMap<String, DecodedImage>>,
    in_flight: Mutex<HashSet<String>>,
    listeners: Mutex<HashMap<String, Vec<Box<dyn FnOnce(DecodedImage) + Send + 'static>>>>,
}

static CACHE_STATE: LazyLock<ImageCacheState> = LazyLock::new(|| ImageCacheState {
    memory_cache: RwLock::new(HashMap::new()),
    in_flight: Mutex::new(HashSet::new()),
    listeners: Mutex::new(HashMap::new()),
});

/// Returns the on-disk cache directory for downloaded textures and images.
pub fn get_texture_cache_dir() -> PathBuf {
    let dir = Config::get_cache_dir().join("textures");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Generates a deterministic filename for a given URL based on its SHA-256 hash.
fn cache_key(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hex::encode(hasher.finalize())
}

/// Computes the average RGB color of non-transparent pixels (useful for backdrop tinting).
fn calculate_average_color(rgba: &image::RgbaImage) -> Option<(u8, u8, u8)> {
    let mut r_sum = 0u64;
    let mut g_sum = 0u64;
    let mut b_sum = 0u64;
    let mut count = 0u64;

    for pixel in rgba.pixels() {
        if pixel[3] > 30 {
            r_sum += pixel[0] as u64;
            g_sum += pixel[1] as u64;
            b_sum += pixel[2] as u64;
            count += 1;
        }
    }

    if count > 0 {
        Some((
            (r_sum / count) as u8,
            (g_sum / count) as u8,
            (b_sum / count) as u8,
        ))
    } else {
        None
    }
}

/// Synchronously loads or downloads and decodes an image, caching it both in memory and on disk.
pub fn load_or_fetch_sync(url: &str) -> Option<DecodedImage> {
    // 1. Check in-memory cache
    if let Ok(guard) = CACHE_STATE.memory_cache.read() {
        if let Some(cached) = guard.get(url) {
            return Some(cached.clone());
        }
    }

    let cache_dir = get_texture_cache_dir();
    let file_path = cache_dir.join(format!("{}.cache", cache_key(url)));

    // 2. Check disk cache or download
    let bytes = if file_path.exists() {
        fs::read(&file_path).ok()?
    } else {
        let resp = client().get(url).send().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let data = resp.bytes().ok()?.to_vec();
        let _ = fs::write(&file_path, &data);
        data
    };

    // 3. Decode image
    let img = image::load_from_memory(&bytes).ok()?;
    let width = img.width() as i32;
    let height = img.height() as i32;
    let rgba_img = img.to_rgba8();
    let average_color = calculate_average_color(&rgba_img);
    let decoded = DecodedImage {
        width,
        height,
        rgba: rgba_img.into_raw(),
        average_color,
    };

    // 4. Store in memory cache
    if let Ok(mut guard) = CACHE_STATE.memory_cache.write() {
        guard.insert(url.to_string(), decoded.clone());
    }

    Some(decoded)
}

/// Asynchronously fetches an image with request coalescing, caching, and background decoding.
/// Invokes `callback` on completion.
pub fn fetch_image_async<F>(url: String, callback: F)
where
    F: FnOnce(Option<DecodedImage>) + Send + 'static,
{
    // Fast path: memory cache hit
    if let Ok(guard) = CACHE_STATE.memory_cache.read() {
        if let Some(cached) = guard.get(&url) {
            let res = cached.clone();
            drop(guard);
            callback(Some(res));
            return;
        }
    }

    let mut in_flight_guard = CACHE_STATE.in_flight.lock().unwrap();
    let mut listeners_guard = CACHE_STATE.listeners.lock().unwrap();

    let cb_box: Box<dyn FnOnce(DecodedImage) + Send + 'static> = Box::new(move |img| callback(Some(img)));

    if in_flight_guard.contains(&url) {
        listeners_guard
            .entry(url.clone())
            .or_default()
            .push(cb_box);
        return;
    }

    in_flight_guard.insert(url.clone());
    listeners_guard
        .entry(url.clone())
        .or_default()
        .push(cb_box);

    drop(listeners_guard);
    drop(in_flight_guard);

    let url_clone = url.clone();
    spawn_io(move || {
        let result = load_or_fetch_sync(&url_clone);

        let mut in_flight_guard = CACHE_STATE.in_flight.lock().unwrap();
        let mut listeners_guard = CACHE_STATE.listeners.lock().unwrap();

        in_flight_guard.remove(&url_clone);
        let callbacks = listeners_guard.remove(&url_clone).unwrap_or_default();

        drop(listeners_guard);
        drop(in_flight_guard);

        if let Some(img) = result {
            for cb in callbacks {
                cb(img.clone());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_deterministic() {
        let key1 = cache_key("https://example.com/icon.png");
        let key2 = cache_key("https://example.com/icon.png");
        let key3 = cache_key("https://example.com/other.png");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert_eq!(key1.len(), 64); // SHA-256 hex string
    }

    #[test]
    fn test_calculate_average_color_opaque() {
        let mut img = image::RgbaImage::new(2, 2);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([100, 150, 200, 255]);
        }
        assert_eq!(calculate_average_color(&img), Some((100, 150, 200)));
    }

    #[test]
    fn test_calculate_average_color_transparent() {
        let mut img = image::RgbaImage::new(2, 2);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([255, 255, 255, 0]); // fully transparent
        }
        assert_eq!(calculate_average_color(&img), None);
    }
}
