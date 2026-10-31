//! An uploaded image, decoded once, and every size and format made from it.

use std::io::Cursor;

use fast_image_resize::images::Image;
use fast_image_resize::{
    FilterType, ImageBufferError, PixelType, ResizeAlg, ResizeOptions, Resizer,
};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageFormat, ImageReader,
    RgbaImage, imageops,
};

use crate::{Crop, FocalPoint, Format, MediaError, PictureEdit, Pixels, Turn, Variant};

/// How hard an encoder may squeeze: derived files for pages, or the stored
/// original every derived file is later made from, which loses less.
#[derive(Debug, Clone, Copy)]
enum Quality {
    Derived,
    Kept,
}

impl Quality {
    /// WebP quality from 0 to 100.
    fn webp(self) -> f32 {
        match self {
            Self::Derived => 78.0,
            Self::Kept => 90.0,
        }
    }

    /// JPEG quality from 1 to 100.
    fn jpeg(self) -> u8 {
        match self {
            Self::Derived => 82,
            Self::Kept => 92,
        }
    }
}

/// An original image, decoded to RGBA pixels once, ready to make variants.
#[derive(Debug, Clone)]
pub struct Original {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    source: Format,
    fallback: Format,
}

impl Original {
    /// Decodes `bytes`. The type is read from the bytes, never from a file
    /// name (RULES.md BL-SEC-6). JPEG, PNG and WebP are accepted. A photo
    /// whose EXIF data says it was taken sideways is turned upright, since
    /// nothing made from it keeps that data.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::UnsupportedImage`] for any other type and
    /// [`MediaError::Decode`] for a file that claims a type but is broken.
    pub fn decode(bytes: &[u8]) -> Result<Self, MediaError> {
        let format = image::guess_format(bytes).map_err(|_| MediaError::UnsupportedImage)?;
        let source = match format {
            ImageFormat::Jpeg => Format::Jpeg,
            ImageFormat::Png => Format::Png,
            ImageFormat::WebP => Format::Webp,
            _ => return Err(MediaError::UnsupportedImage),
        };
        let decode = |source| MediaError::Decode { source };
        let mut decoder = ImageReader::with_format(Cursor::new(bytes), format)
            .into_decoder()
            .map_err(decode)?;
        let orientation = decoder.orientation().map_err(decode)?;
        let mut image = DynamicImage::from_decoder(decoder).map_err(decode)?;
        image.apply_orientation(orientation);
        let rgba = image.into_rgba8();
        let see_through = rgba.pixels().any(|pixel| pixel.0[3] < u8::MAX);
        let fallback = match source {
            Format::Webp if !see_through => Format::Jpeg,
            Format::Webp | Format::Png => Format::Png,
            _ => Format::Jpeg,
        };
        Ok(Self {
            width: rgba.width(),
            height: rgba.height(),
            pixels: rgba.into_raw(),
            source,
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
        let size = (variant.width, variant.height);
        self.encode_area(variant.format, size, None, Quality::Derived)
    }

    /// The image cut to the shape of `width` by `height` around `focal`,
    /// as [`Crop::around`] chooses, then resized to that size and encoded
    /// as `format`. Like [`Original::encode`], call it off the async
    /// runtime.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Resize`] or [`MediaError::Encode`] when a
    /// codec fails, which happens only for an impossible size.
    pub fn encode_cropped(
        &self,
        format: Format,
        width: u32,
        height: u32,
        focal: FocalPoint,
    ) -> Result<Vec<u8>, MediaError> {
        let whole = Pixels {
            width: self.width,
            height: self.height,
        };
        let crop = Crop::around(focal, whole, width, height);
        self.encode_area(format, (width, height), Some(crop), Quality::Derived)
    }

    /// The whole image at its own size in its own format, with less loss
    /// than a derived file, to store in place of an upload. Hidden data
    /// such as GPS positions and camera details is gone, and so is
    /// anything else an upload hid after its pixels (RULES.md BL-SEC-6).
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Encode`] when the encoder fails.
    pub fn reencode(&self) -> Result<Vec<u8>, MediaError> {
        let size = (self.width, self.height);
        self.encode_area(self.source, size, None, Quality::Kept)
    }

    /// The image turned, then cut to the box `edit` keeps. Like
    /// [`Original::encode`], call it off the async runtime.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::ResizeBuffer`] when the pixels do not make an
    /// image, which never happens for a decoded original.
    pub fn edited(&self, edit: PictureEdit) -> Result<Self, MediaError> {
        let whole = RgbaImage::from_raw(self.width, self.height, self.pixels.clone()).ok_or(
            MediaError::ResizeBuffer(ImageBufferError::InvalidBufferSize),
        )?;
        let turned = match edit.turn {
            Turn::None => whole,
            Turn::Quarter => imageops::rotate90(&whole),
            Turn::Half => imageops::rotate180(&whole),
            Turn::ThreeQuarters => imageops::rotate270(&whole),
        };
        let kept = match edit.crop {
            Some(crop) => {
                let size = Pixels {
                    width: turned.width(),
                    height: turned.height(),
                };
                let area = crop.pixels(size);
                imageops::crop_imm(&turned, area.left, area.top, area.width, area.height).to_image()
            }
            None => turned,
        };
        Ok(Self {
            width: kept.width(),
            height: kept.height(),
            pixels: kept.into_raw(),
            ..*self
        })
    }

    /// `crop` of the image, or all of it, resized to `size` and encoded.
    fn encode_area(
        &self,
        format: Format,
        (width, height): (u32, u32),
        crop: Option<Crop>,
        quality: Quality,
    ) -> Result<Vec<u8>, MediaError> {
        let pixels = self.resize(width, height, crop)?;
        let encoded = match format {
            Format::Webp => {
                return webp::Encoder::from_rgba(&pixels, width, height)
                    .encode_simple(false, quality.webp())
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
                JpegEncoder::new_with_quality(&mut out, quality.jpeg())
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
        encoded.map_err(|source| MediaError::Encode { format, source })
    }

    /// `crop` of the pixels, or all of them, at `width` by `height`, with
    /// Lanczos3, which keeps text and edges sharp when shrinking.
    fn resize(&self, width: u32, height: u32, crop: Option<Crop>) -> Result<Vec<u8>, MediaError> {
        if crop.is_none() && width == self.width && height == self.height {
            return Ok(self.pixels.clone());
        }
        let source = Image::from_vec_u8(
            self.width,
            self.height,
            self.pixels.clone(),
            PixelType::U8x4,
        )?;
        let mut target = Image::new(width, height, PixelType::U8x4);
        let mut options =
            ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
        if let Some(crop) = crop {
            let (left, top) = (f64::from(crop.left), f64::from(crop.top));
            options = options.crop(left, top, f64::from(crop.width), f64::from(crop.height));
        }
        Resizer::new().resize(&source, &mut target, &options)?;
        Ok(target.into_vec())
    }
}
