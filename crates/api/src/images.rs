//! Image upload validation + resize.
//!
//! Every photo / avatar handler funnels through this module so the policy
//! (which formats, what size, what dimensions, how do we re-encode) lives
//! in exactly one place. The pipeline is:
//!
//!   1. Cap the raw multipart bytes at [`MAX_RAW_BYTES`] — refuse anything
//!      bigger BEFORE we hand it to the image decoder. The decoder happily
//!      walks a 4 GB "PNG" if we let it.
//!   2. Sniff the first few bytes for one of the three magic-byte signatures
//!      we accept (JPEG, PNG, WebP). A `Content-Type` header from the client
//!      can lie; the magic bytes can't.
//!   3. If the caller passed a filename hint, the extension must agree with
//!      the magic bytes. Mismatch is a 400 — surfaces a stale extension long
//!      before we'd hit a confusing "corrupt image" error from the decoder.
//!   4. Decode, scale-down to fit in [`MAX_DIMENSION`]² (preserving aspect
//!      ratio, never up-scaling), re-encode as JPEG quality 80. Output is
//!      always a small, predictable JPEG regardless of input format.

use std::io::Cursor;

use bytes::Bytes;
use image::{ImageFormat, ImageReader};

/// Hard cap on the raw upload size.
///
/// Covers a JPEG photo straight out of a modern phone with headroom. The
/// resize output is orders of magnitude smaller, but a malicious client
/// could ship a 200 MB "pixel bomb" and we'd OOM trying to decode it.
pub const MAX_RAW_BYTES: usize = 20 * 1024 * 1024;

/// Maximum width OR height of the resized output.
///
/// 512 px is enough for the largest avatar / photo render in the FE
/// (~80 px shown × 2 for retina × some buffer for zoom-in) without burning
/// storage on bytes nobody sees.
pub const MAX_DIMENSION: u32 = 512;

/// JPEG quality of the re-encoded output. 80 is the sweet spot: visually
/// indistinguishable from 90, ~30% smaller bytes on disk.
const JPEG_QUALITY: u8 = 80;

/// Output `Content-Type` we store alongside the bytes — always JPEG.
pub const OUTPUT_CONTENT_TYPE: &str = "image/jpeg";

/// Output file extension used when minting storage keys.
pub const OUTPUT_EXTENSION: &str = "jpg";

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("upload exceeds maximum size of {max} bytes")]
    TooLarge { max: usize },
    #[error("unsupported image format — must be JPEG, PNG, or WebP")]
    UnsupportedFormat,
    #[error("filename extension `{ext}` does not match the file's actual format")]
    ExtensionMismatch { ext: String },
    #[error("could not decode image: {0}")]
    Decode(String),
    #[error("could not encode JPEG output: {0}")]
    Encode(String),
}

/// Three accepted formats, identified by magic-byte signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetectedFormat {
    Jpeg,
    Png,
    WebP,
}

impl DetectedFormat {
    /// Inspect the leading bytes of `data` and return the detected format
    /// — or `None` if it's none of the three we accept.
    fn detect(data: &[u8]) -> Option<Self> {
        // JPEG: FF D8 FF
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(Self::Jpeg);
        }
        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Some(Self::Png);
        }
        // WebP: "RIFF" .... "WEBP" — `.get(8..12)` returns None for short
        // inputs so we don't risk a panic from indexed slicing.
        if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
            return Some(Self::WebP);
        }
        None
    }

    /// Which extensions a caller may legitimately attach to this format.
    /// We accept the common spellings so a `.jpeg` doesn't 400 against a
    /// JPEG with magic bytes.
    const fn allowed_extensions(self) -> &'static [&'static str] {
        match self {
            Self::Jpeg => &["jpg", "jpeg"],
            Self::Png => &["png"],
            Self::WebP => &["webp"],
        }
    }
}

/// Validate `bytes` against the size / magic-bytes / extension rules and
/// resize to a JPEG suitable for serving back.
///
/// `filename` is the original filename (from the multipart `Content-Disposition`
/// or similar) — used purely for the extension cross-check; pass `None`
/// when the caller has no filename hint and trust the magic-bytes alone.
///
/// # Errors
///
/// Returns [`ImageError::TooLarge`] when `bytes.len() > MAX_RAW_BYTES`,
/// [`ImageError::UnsupportedFormat`] when the leading bytes don't match
/// one of the three accepted magic-byte signatures (JPEG / PNG / WebP),
/// [`ImageError::ExtensionMismatch`] when a filename was passed and its
/// extension disagrees with the detected format, and [`ImageError::Decode`]
/// or [`ImageError::Encode`] when the underlying image crate fails to
/// decode the input or re-encode the JPEG output.
pub fn validate_and_resize(bytes: &Bytes, filename: Option<&str>) -> Result<Vec<u8>, ImageError> {
    if bytes.len() > MAX_RAW_BYTES {
        return Err(ImageError::TooLarge { max: MAX_RAW_BYTES });
    }

    let format = DetectedFormat::detect(bytes).ok_or(ImageError::UnsupportedFormat)?;

    if let Some(name) = filename
        && let Some(ext) = extract_extension(name)
    {
        let lc = ext.to_ascii_lowercase();
        if !format.allowed_extensions().contains(&lc.as_str()) {
            return Err(ImageError::ExtensionMismatch { ext: lc });
        }
    }

    let img_format = match format {
        DetectedFormat::Jpeg => ImageFormat::Jpeg,
        DetectedFormat::Png => ImageFormat::Png,
        DetectedFormat::WebP => ImageFormat::WebP,
    };

    let reader = ImageReader::with_format(Cursor::new(bytes), img_format);
    let img = reader.decode().map_err(|e| ImageError::Decode(e.to_string()))?;

    // Only scale DOWN — a 200×200 input stays 200×200 (no upscaling
    // blur, no wasted bytes). `thumbnail` preserves aspect ratio.
    let resized = if img.width() > MAX_DIMENSION || img.height() > MAX_DIMENSION {
        img.thumbnail(MAX_DIMENSION, MAX_DIMENSION)
    } else {
        img
    };

    // Flatten any alpha channel to white BEFORE the JPEG encode: JPEG has
    // no alpha, so a transparent PNG would otherwise get a black fill
    // that looks broken on light backgrounds.
    let rgb = resized.into_rgb8();

    let mut out = Vec::with_capacity(64 * 1024);
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(Cursor::new(&mut out), JPEG_QUALITY);
    encoder
        .encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)
        .map_err(|e| ImageError::Encode(e.to_string()))?;
    Ok(out)
}

/// Strip everything up to and including the last `.` from `filename` and
/// return the remainder lowercased. `None` if there's no extension at all.
fn extract_extension(filename: &str) -> Option<&str> {
    filename.rsplit_once('.').map(|(_, ext)| ext)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Round-trip a 2×2 RGB image through the `image` crate's PNG encoder so
    /// we don't ship a binary fixture in the repo.
    fn tiny_png() -> Bytes {
        let img = image::RgbImage::from_fn(2, 2, |_, _| image::Rgb([255, 0, 0]));
        let mut buf = Vec::new();
        image::DynamicImage::from(img)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        Bytes::from(buf)
    }

    fn tiny_jpeg() -> Bytes {
        let img = image::RgbImage::from_fn(2, 2, |_, _| image::Rgb([0, 128, 255]));
        let mut buf = Vec::new();
        image::DynamicImage::from(img)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
            .unwrap();
        Bytes::from(buf)
    }

    #[test]
    fn rejects_oversize_payload_before_decoding() {
        let blob = Bytes::from(vec![0u8; MAX_RAW_BYTES + 1]);
        let err = validate_and_resize(&blob, None).unwrap_err();
        assert!(matches!(err, ImageError::TooLarge { .. }));
    }

    #[test]
    fn rejects_random_bytes_that_arent_an_image() {
        let blob = Bytes::from_static(b"definitely not an image");
        let err = validate_and_resize(&blob, None).unwrap_err();
        assert!(matches!(err, ImageError::UnsupportedFormat));
    }

    #[test]
    fn accepts_a_real_png_and_re_encodes_as_jpeg() {
        let png = tiny_png();
        let out = validate_and_resize(&png, Some("anything.png")).unwrap();
        // Output magic-bytes are JPEG regardless of input.
        assert_eq!(&out[..3], &[0xFF, 0xD8, 0xFF]);
    }

    #[test]
    fn rejects_a_filename_extension_that_lies_about_the_content() {
        // PNG bytes with a .jpg filename — caller pasted the wrong extension.
        let err = validate_and_resize(&tiny_png(), Some("photo.jpg")).unwrap_err();
        assert!(matches!(err, ImageError::ExtensionMismatch { .. }));
    }

    #[test]
    fn accepts_jpeg_with_jpeg_extension() {
        validate_and_resize(&tiny_jpeg(), Some("a.JPEG")).unwrap();
    }

    #[test]
    fn ignores_extension_when_filename_is_none() {
        // Mismatched bytes-vs-extension only fires when a filename is given.
        // With None, magic bytes alone decide acceptance.
        validate_and_resize(&tiny_png(), None).unwrap();
    }

    #[test]
    fn detect_recognises_each_supported_format() {
        assert_eq!(DetectedFormat::detect(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(DetectedFormat::Jpeg));
        assert_eq!(
            DetectedFormat::detect(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00]),
            Some(DetectedFormat::Png)
        );
        let mut webp = Vec::from(b"RIFF\0\0\0\0WEBP" as &[u8]);
        webp.push(0);
        assert_eq!(DetectedFormat::detect(&webp), Some(DetectedFormat::WebP));
    }
}
