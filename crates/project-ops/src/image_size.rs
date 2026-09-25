//! Dispatches on file signature, not the claimed extension — a
//! mislabeled upload still measures correctly. Port of
//! packages/project-ops/src/imageSize.ts.

pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

/// PNG: 8-byte signature, then an IHDR chunk: 4-byte length, "IHDR",
/// 4-byte width, 4-byte height (big-endian).
fn read_png_size(bytes: &[u8]) -> Option<ImageSize> {
    if bytes.len() < 24 {
        return None;
    }
    if &bytes[12..16] != b"IHDR" {
        return None;
    }
    Some(ImageSize {
        width: u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
        height: u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
    })
}

/// JPEG: SOI (0xFFD8), then a run of marker segments. Markers with no
/// length field (SOI/EOI/RSTn/TEM) are skipped bare; every other
/// marker is followed by a 2-byte big-endian segment length. The SOF
/// marker (0xFFC0-0xFFCF, excluding DHT/JPG/DAC at C4/C8/CC) carries
/// height then width as two big-endian shorts, right after a 1-byte
/// precision.
fn read_jpeg_size(bytes: &[u8]) -> Option<ImageSize> {
    if bytes.len() < 4 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return None;
    }

    let mut offset = 2usize;
    while offset + 1 < bytes.len() {
        if bytes[offset] != 0xff {
            offset += 1;
            continue;
        }
        let marker = bytes[offset + 1];

        if marker == 0xd8 || marker == 0xd9 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            offset += 2;
            continue;
        }

        if offset + 4 > bytes.len() {
            return None;
        }
        let length = u16::from_be_bytes(bytes[offset + 2..offset + 4].try_into().unwrap()) as usize;
        let is_sof =
            (0xc0..=0xcf).contains(&marker) && marker != 0xc4 && marker != 0xc8 && marker != 0xcc;

        if is_sof {
            if offset + 9 > bytes.len() {
                return None;
            }
            return Some(ImageSize {
                height: u16::from_be_bytes(bytes[offset + 5..offset + 7].try_into().unwrap())
                    as u32,
                width: u16::from_be_bytes(bytes[offset + 7..offset + 9].try_into().unwrap()) as u32,
            });
        }

        offset += 2 + length;
    }
    None
}

pub fn read_image_size(bytes: &[u8]) -> Option<ImageSize> {
    read_png_size(bytes).or_else(|| read_jpeg_size(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; 24];
        bytes[12..16].copy_from_slice(b"IHDR");
        bytes[16..20].copy_from_slice(&width.to_be_bytes());
        bytes[20..24].copy_from_slice(&height.to_be_bytes());
        bytes
    }

    fn fake_jpeg(width: u16, height: u16) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8, 0xff, 0xc0, 0, 11, 8];
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&[1, 1, 0x11, 0, 0xff, 0xd9]);
        bytes
    }

    fn size(bytes: &[u8]) -> Option<(u32, u32)> {
        read_image_size(bytes).map(|s| (s.width, s.height))
    }

    #[test]
    fn reads_a_png_ihdr() {
        assert_eq!(size(&fake_png(400, 300)), Some((400, 300)));
    }

    #[test]
    fn reads_a_jpeg_sof0() {
        assert_eq!(size(&fake_jpeg(640, 480)), Some((640, 480)));
    }

    #[test]
    fn skips_a_leading_app0_segment() {
        let jpeg = fake_jpeg(200, 100);
        let mut with_app0 = jpeg[..2].to_vec();
        with_app0.extend_from_slice(&[0xff, 0xe0, 0x00, 0x04, 0x00, 0x00]);
        with_app0.extend_from_slice(&jpeg[2..]);
        assert_eq!(size(&with_app0), Some((200, 100)));
    }

    #[test]
    fn rejects_short_or_unknown_buffers() {
        assert_eq!(size(&[0, 0, 0]), None);
        assert_eq!(size(b"not an image, just text"), None);
    }
}
