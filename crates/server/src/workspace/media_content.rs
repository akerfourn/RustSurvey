use super::AppError;
use axum::http::StatusCode;
use rustsurvey_object_store::ByteRange;

fn dimensions(width: u32, height: u32) -> Result<(), AppError> {
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > 32_000_000
    {
        return Err(invalid_image());
    }
    Ok(())
}
fn invalid_image() -> AppError {
    AppError::new(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "invalid_image",
        "Image PNG ou JPEG requise, limitée à 8192 pixels par côté et 32 millions de pixels.",
    )
}

/// Inspect the signature and declared dimensions; decoding remains the browser's responsibility.
pub(super) fn content_type(bytes: &[u8]) -> Result<&'static str, AppError> {
    if bytes.len() >= 33
        && &bytes[..8] == b"\x89PNG\r\n\x1a\n"
        && bytes[8..12] == 13_u32.to_be_bytes()
        && &bytes[12..16] == b"IHDR"
    {
        let width = u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| invalid_image())?);
        let height = u32::from_be_bytes(bytes[20..24].try_into().map_err(|_| invalid_image())?);
        dimensions(width, height)?;
        return Ok("image/png");
    }
    if !bytes.starts_with(b"\xff\xd8") {
        return Err(invalid_image());
    }
    let mut position = 2;
    while position + 4 <= bytes.len() {
        if bytes[position] != 0xff {
            return Err(invalid_image());
        }
        position += 1;
        while bytes.get(position) == Some(&0xff) {
            position += 1;
        }
        let marker = *bytes.get(position).ok_or_else(invalid_image)?;
        position += 1;
        if marker == 0xd9 || marker == 0xda {
            break;
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let length = bytes
            .get(position..position + 2)
            .ok_or_else(invalid_image)?;
        let length = usize::from(u16::from_be_bytes([length[0], length[1]]));
        if length < 2 || position + length > bytes.len() {
            return Err(invalid_image());
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            if length < 8 {
                return Err(invalid_image());
            }
            let height = u32::from(u16::from_be_bytes([
                bytes[position + 3],
                bytes[position + 4],
            ]));
            let width = u32::from(u16::from_be_bytes([
                bytes[position + 5],
                bytes[position + 6],
            ]));
            dimensions(width, height)?;
            return Ok("image/jpeg");
        }
        position += length;
    }
    Err(invalid_image())
}

pub(super) fn range(header: &str, size: u64) -> Result<ByteRange, AppError> {
    let invalid = || {
        AppError::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "invalid_range",
            "Plage d’octets invalide ou multiple.",
        )
    };
    let (start, end) = header
        .strip_prefix("bytes=")
        .and_then(|value| value.split_once('-'))
        .ok_or_else(invalid)?;
    if !start.bytes().all(|byte| byte.is_ascii_digit())
        || !end.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    if start.is_empty() {
        let count = end.parse::<u64>().map_err(|_| invalid())?;
        if count == 0 || size == 0 {
            return Err(invalid());
        }
        return Ok(ByteRange {
            start: size.saturating_sub(count),
            end: size,
        });
    }
    let start = start.parse::<u64>().map_err(|_| invalid())?;
    let end = if end.is_empty() {
        size
    } else {
        end.parse::<u64>()
            .map_err(|_| invalid())?
            .saturating_add(1)
            .min(size)
    };
    if start >= size || start >= end {
        return Err(invalid());
    }
    Ok(ByteRange { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_byte_ranges_should_support_suffixes_and_reject_multiple_or_signed_ranges() {
        assert_eq!(
            range("bytes=-3", 10).unwrap(),
            ByteRange { start: 7, end: 10 }
        );
        assert_eq!(
            range("bytes=4-100", 10).unwrap(),
            ByteRange { start: 4, end: 10 }
        );
        for input in [
            "bytes=+1-2",
            "bytes=0-1,4-5",
            "bytes=-0",
            "bytes=10-",
            "bytes=8-4",
            "bytes=0-184467440737095516160",
            "bytes= 0-1",
        ] {
            let error = range(input, 10).unwrap_err();
            assert_eq!(error.status, StatusCode::RANGE_NOT_SATISFIABLE);
            assert_eq!(error.code, "invalid_range");
        }
    }
    #[test]
    fn test_image_headers_should_refuse_extreme_dimensions_before_browser_decoding() {
        let mut pixel = include_bytes!("../../tests/fixtures/pixel.png").to_vec();
        assert_eq!(content_type(&pixel).unwrap(), "image/png");
        pixel[16..20].copy_from_slice(&8193_u32.to_be_bytes());
        assert_eq!(content_type(&pixel).unwrap_err().code, "invalid_image");
        assert_eq!(
            content_type(b"\xff\xd8\xff\xc0\x00\xff").unwrap_err().code,
            "invalid_image"
        );
    }
}
