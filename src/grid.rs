use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use base64::{Engine as _, engine::general_purpose};
use rayon::prelude::*;

#[derive(Debug)]
pub enum GridError {
    ImageDecodeError(image::ImageError),
    EmptyInput,
    DownloadError(reqwest::Error),
    Base64DecodeError(base64::DecodeError),
    Utf8Error(std::string::FromUtf8Error),
}

#[derive(Debug)]
pub struct DownloadResult {
    pub url: String,
    pub data: Vec<u8>,
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridError::ImageDecodeError(e) => write!(f, "Failed to decode image: {}", e),
            GridError::EmptyInput => write!(f, "No images provided"),
            GridError::DownloadError(e) => write!(f, "Failed to download image: {}", e),
            GridError::Base64DecodeError(e) => write!(f, "Failed to decode base64: {}", e),
            GridError::Utf8Error(e) => write!(f, "Invalid UTF-8 in decoded data: {}", e),
        }
    }
}

impl std::error::Error for GridError {}

impl From<image::ImageError> for GridError {
    fn from(err: image::ImageError) -> Self {
        GridError::ImageDecodeError(err)
    }
}

impl From<reqwest::Error> for GridError {
    fn from(err: reqwest::Error) -> Self {
        GridError::DownloadError(err)
    }
}

impl From<base64::DecodeError> for GridError {
    fn from(err: base64::DecodeError) -> Self {
        GridError::Base64DecodeError(err)
    }
}

impl From<std::string::FromUtf8Error> for GridError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        GridError::Utf8Error(err)
    }
}

pub fn create_image_grid(image_bytes: &[&[u8]]) -> Result<RgbaImage, GridError> {
    if image_bytes.is_empty() {
        return Err(GridError::EmptyInput);
    }

    let images: Result<Vec<RgbaImage>, GridError> = image_bytes
        .par_iter()
        .map(|bytes| {
            image::load_from_memory(bytes)
                .map(|img| img.to_rgba8())
                .map_err(GridError::from)
        })
        .collect();

    let images = images?;

    let (cols, rows) = calculate_grid_dimensions(images.len());

    let max_width = images.iter().map(|img| img.width()).max().unwrap_or(0);
    let max_height = images.iter().map(|img| img.height()).max().unwrap_or(0);

    let grid_width = cols * max_width;
    let grid_height = rows * max_height;

    let mut grid_image: RgbaImage = ImageBuffer::from_pixel(
        grid_width,
        grid_height,
        Rgba([255, 255, 255, 255]),
    );

    for (idx, img) in images.iter().enumerate() {
        let col = (idx as u32) % cols;
        let row = (idx as u32) / cols;

        let x_offset = col * max_width;
        let y_offset = row * max_height;

        image::imageops::overlay(&mut grid_image, img, x_offset as i64, y_offset as i64);
    }

    Ok(grid_image)
}

fn calculate_grid_dimensions(count: usize) -> (u32, u32) {
    match count {
        0 => (0, 0),
        1 => (1, 1),
        2..=4 => (2, 2),
        n => {
            let cols = 3;
            let rows = (n as f32 / cols as f32).ceil() as u32;
            (cols, rows)
        }
    }
}

pub struct ImageService {
    client: reqwest::Client,
}

impl ImageService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn download_images(&self, urls: &[String]) -> Result<Vec<Vec<u8>>, GridError> {
        if urls.is_empty() {
            return Err(GridError::EmptyInput);
        }

        let download_tasks: Vec<_> = urls
            .iter()
            .map(|url| {
                let client = self.client.clone();
                let url = url.clone();
                async move {
                    let response = client.get(&url).send().await?;
                    let bytes = response.bytes().await?;
                    Ok::<Vec<u8>, reqwest::Error>(bytes.to_vec())
                }
            })
            .collect();

        let results = futures::future::join_all(download_tasks).await;

        let images: Result<Vec<Vec<u8>>, GridError> = results
            .into_iter()
            .map(|r| r.map_err(GridError::from))
            .collect();

        images
    }

    pub async fn process_base64_urls(&self, base64_urls: &str) -> Result<RgbaImage, GridError> {
        let decoded_bytes = general_purpose::STANDARD
            .decode(base64_urls)
            .or_else(|_| general_purpose::URL_SAFE_NO_PAD.decode(base64_urls))
            .or_else(|_| general_purpose::URL_SAFE.decode(base64_urls))?;
        let decoded_str = String::from_utf8(decoded_bytes)?;

        let urls: Vec<String> = decoded_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let downloaded = self.download_images(&urls).await?;

        let refs: Vec<&[u8]> = downloaded.iter().map(|v| v.as_slice()).collect();

        create_image_grid(&refs)
    }
}

impl Default for ImageService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_dimensions() {
        assert_eq!(calculate_grid_dimensions(0), (0, 0));
        assert_eq!(calculate_grid_dimensions(1), (1, 1));
        assert_eq!(calculate_grid_dimensions(2), (2, 2));
        assert_eq!(calculate_grid_dimensions(3), (2, 2));
        assert_eq!(calculate_grid_dimensions(4), (2, 2));
        assert_eq!(calculate_grid_dimensions(5), (3, 2));
        assert_eq!(calculate_grid_dimensions(6), (3, 2));
        assert_eq!(calculate_grid_dimensions(7), (3, 3));
        assert_eq!(calculate_grid_dimensions(9), (3, 3));
        assert_eq!(calculate_grid_dimensions(10), (3, 4));
    }

    #[test]
    fn test_empty_input() {
        let result = create_image_grid(&[]);
        assert!(matches!(result, Err(GridError::EmptyInput)));
    }
}
