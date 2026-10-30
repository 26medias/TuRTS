//! Header-only media metadata used by MC validation; no image or audio decoder is loaded.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PngHeader {
    pub width: u32,
    pub height: u32,
}

pub fn png_header(bytes: &[u8]) -> Option<PngHeader> {
    if bytes.get(..8)? != b"\x89PNG\r\n\x1a\n"
        || bytes.get(8..12)? != 13u32.to_be_bytes()
        || bytes.get(12..16)? != b"IHDR"
    {
        return None;
    }
    let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    (width > 0 && height > 0).then_some(PngHeader { width, height })
}
pub fn font_header(bytes: &[u8]) -> bool {
    matches!(bytes.get(..4), Some([0, 1, 0, 0]) | Some(b"OTTO"))
}
pub fn ogg_duration_seconds(bytes: &[u8]) -> Option<f64> {
    let mut offset = 0usize;
    let mut packet = Vec::new();
    let mut identification = None;
    let mut last_granule = None;
    while offset < bytes.len() {
        let header = bytes.get(offset..offset.checked_add(27)?)?;
        if header.get(..4)? != b"OggS" || header[4] != 0 {
            return None;
        }
        let granule = u64::from_le_bytes(header[6..14].try_into().ok()?);
        let segments = usize::from(header[26]);
        let lacing = bytes.get(offset.checked_add(27)?..offset.checked_add(27 + segments)?)?;
        let body_len = lacing
            .iter()
            .try_fold(0usize, |sum, &part| sum.checked_add(usize::from(part)))?;
        let body_start = offset.checked_add(27 + segments)?;
        let body = bytes.get(body_start..body_start.checked_add(body_len)?)?;
        if granule != u64::MAX {
            last_granule = Some(granule);
        }
        let mut packet_offset = 0usize;
        for &part in lacing {
            let part = usize::from(part);
            packet.extend_from_slice(body.get(packet_offset..packet_offset.checked_add(part)?)?);
            packet_offset += part;
            if part < 255 && identification.is_none() {
                if packet.get(..7)? != b"\x01vorbis" || packet.len() < 30 {
                    return None;
                }
                let sample_rate = u32::from_le_bytes(packet[12..16].try_into().ok()?);
                if sample_rate == 0 {
                    return None;
                }
                identification = Some(sample_rate);
            }
            if part < 255 && identification.is_some() {
                packet.clear();
            }
        }
        offset = body_start.checked_add(body_len)?;
    }
    let sample_rate = identification?;
    let granule = last_granule?;
    Some(granule as f64 / f64::from(sample_rate))
}

#[cfg(test)]
mod tests {
    use super::{font_header, ogg_duration_seconds, png_header};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0]);
        bytes
    }

    fn ogg(seconds: u64) -> Vec<u8> {
        let mut packet = b"\x01vorbis".to_vec();
        packet.extend_from_slice(&0u32.to_le_bytes());
        packet.push(2);
        packet.extend_from_slice(&48_000u32.to_le_bytes());
        packet.extend_from_slice(&[0; 12]);
        packet.extend_from_slice(&[0xB8, 1]);
        let mut first = b"OggS\0\x02".to_vec();
        first.extend_from_slice(&0u64.to_le_bytes());
        first.extend_from_slice(&1u32.to_le_bytes());
        first.extend_from_slice(&0u32.to_le_bytes());
        first.extend_from_slice(&[0; 4]);
        first.push(1);
        first.push(packet.len() as u8);
        first.extend_from_slice(&packet);
        let mut last = b"OggS\0\x04".to_vec();
        last.extend_from_slice(&(48_000 * seconds).to_le_bytes());
        last.extend_from_slice(&1u32.to_le_bytes());
        last.extend_from_slice(&1u32.to_le_bytes());
        last.extend_from_slice(&[0; 4]);
        last.push(0);
        first.extend(last);
        first
    }

    #[test]
    fn png_header_reads_only_ihdr_dimensions() {
        assert_eq!(
            png_header(&png(2048, 1024)),
            Some(super::PngHeader {
                width: 2048,
                height: 1024
            })
        );
        assert_eq!(png_header(b"not a png"), None);
    }

    #[test]
    fn font_header_accepts_ttf_and_otf_signatures() {
        assert!(font_header(&[0, 1, 0, 0]));
        assert!(font_header(b"OTTO"));
        assert!(!font_header(b"bad!"));
    }

    #[test]
    fn ogg_vorbis_duration_uses_last_page_granule_and_sample_rate() {
        assert_eq!(ogg_duration_seconds(&ogg(31)), Some(31.0));
        assert_eq!(ogg_duration_seconds(b"OggS"), None);
    }
}
