use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use std::io::Cursor;

#[derive(Debug)]
pub enum GridError {
    ImageDecodeError(image::ImageError),
    EmptyInput,
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridError::ImageDecodeError(e) => write!(f, "Failed to decode image: {}", e),
            GridError::EmptyInput => write!(f, "No images provided"),
        }
    }
}

impl std::error::Error for GridError {}

impl From<image::ImageError> for GridError {
    fn from(err: image::ImageError) -> Self {
        GridError::ImageDecodeError(err)
    }
}

pub fn create_image_grid(image_bytes: &[&[u8]]) -> Result<RgbaImage, GridError> {
    if image_bytes.is_empty() {
        return Err(GridError::EmptyInput);
    }

    let images: Result<Vec<DynamicImage>, GridError> = image_bytes
        .iter()
        .map(|bytes| {
            image::load_from_memory(bytes).map_err(GridError::from)
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

        let rgba_img = img.to_rgba8();

        image::imageops::overlay(&mut grid_image, &rgba_img, x_offset as i64, y_offset as i64);
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
