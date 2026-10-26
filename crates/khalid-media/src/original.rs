//! An uploaded image, decoded once, and every size and format made from it.

use std::io::Cursor;

use fast_image_resize::images::Image;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageFormat};

use crate::{Format, MediaError, Variant};

/// WebP quality from 0 to 100.
const WEBP_QUALITY: f32 = 78.0;
/// JPEG quality from 1 to 100 for the fallback files.
const JPEG_QUALITY: u8 = 82;

/// An original image, decoded to RGBA pixels once, ready to make variants.
#[derive(Debug, Clone)]
pub struct Original {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    fallback: Format,
}

impl Original {
    /// Decodes `bytes`. The type is read from the bytes, never from a file
    /// name (RULES.md BL-SEC-6). JPEG, PNG and WebP are accepted.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::UnsupportedImage`] for any other type and
    /// [`MediaError::Decode`] for a file that claims a type but is broken.
    pub fn decode(bytes: &[u8]) -> Result<Self, MediaError> {
        let format = image::guess_format(bytes).map_err(|_| MediaError::UnsupportedImage)?;
        if !matches!(
            format,
            ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP
        ) {
            return Err(MediaError::UnsupportedImage);
        }
        let decoded = image::load_from_memory_with_format(bytes, format)
            .map_err(|source| MediaError::Decode { source })?;
        let rgba = decoded.into_rgba8();
        let see_through = rgba.pixels().any(|pixel| pixel.0[3] < u8::MAX);
        let fallback = match format {
            ImageFormat::Jpeg => Format::Jpeg,
            ImageFormat::WebP if !see_through => Format::Jpeg,
            _ => Format::Png,
        };
        Ok(Self {
            width: rgba.width(),
            height: rgba.height(),
            pixels: rgba.into_raw(),
            fallback,
        })
    }

    /// The width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The format older browsers get: the original's own JPEG or PNG, and
    /// for a WebP original, PNG when any pixel is see-through, else JPEG.
    pub fn fallback(&self) -> Format {
        self.fallback
    }

    /// The variant of `format` at `width`, with the height that keeps the
    /// original's shape, rounded to the nearest pixel and at least 1.
    pub fn variant(&self, format: Format, width: u32) -> Variant {
        let scaled = (u64::from(self.height) * u64::from(width) + u64::from(self.width) / 2)
            .checked_div(u64::from(self.width))
            .unwrap_or(0);
        let height = u32::try_from(scaled).unwrap_or(u32::MAX).max(1);
        Variant {
            format,
            width,
            height,
        }
    }

    /// Resizes to `variant`'s size and encodes in its format. This is
    /// CPU-heavy: call it from a worker pool, never on the async runtime
    /// (RULES.md BL-RUN-7).
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Resize`] or [`MediaError::Encode`] when a
    /// codec fails, which happens only for an impossible size.
    pub fn encode(&self, variant: &Variant) -> Result<Vec<u8>, MediaError> {
        let pixels = self.resize(variant.width, variant.height)?;
        let (width, height) = (variant.width, variant.height);
        let encoded = match variant.format {
            Format::Webp => {
                return webp::Encoder::from_rgba(&pixels, width, height)
                    .encode_simple(false, WEBP_QUALITY)
                    .map(|memory| memory.to_vec())
                    .map_err(|error| MediaError::EncodeWebp {
                        code: format!("{error:?}"),
                    });
            }
            Format::Jpeg => {
                let (rgba, _) = pixels.as_chunks::<4>();
                let rgb: Vec<u8> = rgba
                    .iter()
                    .flat_map(|&[red, green, blue, _]| [red, green, blue])
                    .collect();
                let mut out = Cursor::new(Vec::new());
                JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
                    .write_image(&rgb, width, height, ExtendedColorType::Rgb8)
                    .map(|()| out.into_inner())
            }
            Format::Png => {
                let mut out = Vec::new();
                PngEncoder::new(&mut out)
                    .write_image(&pixels, width, height, ExtendedColorType::Rgba8)
                    .map(|()| out)
            }
        };
        encoded.map_err(|source| MediaError::Encode {
            format: variant.format,
            source,
        })
    }

    /// The pixels at `width` by `height`, with Lanczos3, which keeps text
    /// and edges sharp when shrinking.
    fn resize(&self, width: u32, height: u32) -> Result<Vec<u8>, MediaError> {
        if width == self.width && height == self.height {
            return Ok(self.pixels.clone());
        }
        let source = Image::from_vec_u8(
            self.width,
            self.height,
            self.pixels.clone(),
            PixelType::U8x4,
        )?;
        let mut target = Image::new(width, height, PixelType::U8x4);
        let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
        Resizer::new().resize(&source, &mut target, &options)?;
        Ok(target.into_vec())
    }
}
