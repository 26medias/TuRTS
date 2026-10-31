//! Header-only media metadata used by MC validation; no image or audio decoder is loaded.

use std::collections::BTreeMap;

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
    #[derive(Default)]
    struct Stream {
        packet: Vec<u8>,
        sample_rate: Option<u32>,
        final_granule: Option<u64>,
        expected_sequence: u32,
        ended: bool,
    }

    let mut offset = 0usize;
    let mut streams = BTreeMap::<u32, Stream>::new();
    let mut active_serial = None;
    while offset < bytes.len() {
        let header = bytes.get(offset..offset.checked_add(27)?)?;
        if header.get(..4)? != b"OggS" || header[4] != 0 {
            return None;
        }
        let serial = u32::from_le_bytes(header[14..18].try_into().ok()?);
        let flags = header[5];
        let sequence = u32::from_le_bytes(header[18..22].try_into().ok()?);
        let is_bos = flags & 0x02 != 0;
        let is_eos = flags & 0x04 != 0;
        if flags & !0x07 != 0 {
            return None;
        }
        if let Some(stream) = streams.get_mut(&serial) {
            if stream.ended || is_bos || sequence != stream.expected_sequence {
                // Serial reuse and page-sequence discontinuities are not valid supported chains.
                return None;
            }
            stream.expected_sequence = sequence.checked_add(1)?;
        } else {
            if !is_bos || sequence != 0 || active_serial.is_some() {
                // Accept sequentially chained Vorbis streams; reject multiplexed/ambiguous topology.
                return None;
            }
            active_serial = Some(serial);
            streams.insert(
                serial,
                Stream {
                    expected_sequence: 1,
                    ..Stream::default()
                },
            );
        }
        let stream = streams.get_mut(&serial)?;
        let granule = u64::from_le_bytes(header[6..14].try_into().ok()?);
        let segments = usize::from(header[26]);
        let lacing = bytes.get(offset.checked_add(27)?..offset.checked_add(27 + segments)?)?;
        let body_len = lacing
            .iter()
            .try_fold(0usize, |sum, &part| sum.checked_add(usize::from(part)))?;
        let body_start = offset.checked_add(27 + segments)?;
        let body = bytes.get(body_start..body_start.checked_add(body_len)?)?;
        if granule != u64::MAX {
            if stream
                .final_granule
                .is_some_and(|previous| granule < previous)
            {
                return None;
            }
            stream.final_granule = Some(granule);
        }
        let mut packet_offset = 0usize;
        for &part in lacing {
            let part = usize::from(part);
            stream
                .packet
                .extend_from_slice(body.get(packet_offset..packet_offset.checked_add(part)?)?);
            packet_offset += part;
            if part < 255 && stream.sample_rate.is_none() {
                if stream.packet.get(..7)? != b"\x01vorbis" || stream.packet.len() < 30 {
                    return None;
                }
                let sample_rate = u32::from_le_bytes(stream.packet[12..16].try_into().ok()?);
                if sample_rate == 0 {
                    return None;
                }
                stream.sample_rate = Some(sample_rate);
            }
            if part < 255 && stream.sample_rate.is_some() {
                stream.packet.clear();
            }
        }
        if is_eos {
            stream.ended = true;
            active_serial = None;
        }
        offset = body_start.checked_add(body_len)?;
    }
    if streams.is_empty() || active_serial.is_some() {
        return None;
    }
    streams.values().try_fold(0.0, |total, stream| {
        if !stream.ended {
            return None;
        }
        Some(total + stream.final_granule? as f64 / f64::from(stream.sample_rate?))
    })
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
        ogg_with_serial_and_rate(seconds, 1, 48_000)
    }

    fn ogg_with_serial_and_rate(seconds: u64, serial: u32, sample_rate: u32) -> Vec<u8> {
        let mut packet = b"\x01vorbis".to_vec();
        packet.extend_from_slice(&0u32.to_le_bytes());
        packet.push(2);
        packet.extend_from_slice(&sample_rate.to_le_bytes());
        packet.extend_from_slice(&[0; 12]);
        packet.extend_from_slice(&[0xB8, 1]);
        let mut first = b"OggS\0\x02".to_vec();
        first.extend_from_slice(&0u64.to_le_bytes());
        first.extend_from_slice(&serial.to_le_bytes());
        first.extend_from_slice(&0u32.to_le_bytes());
        first.extend_from_slice(&[0; 4]);
        first.push(1);
        first.push(packet.len() as u8);
        first.extend_from_slice(&packet);
        let mut last = b"OggS\0\x04".to_vec();
        last.extend_from_slice(&(u64::from(sample_rate) * seconds).to_le_bytes());
        last.extend_from_slice(&serial.to_le_bytes());
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

    #[test]
    fn chained_vorbis_streams_sum_each_serial_duration() {
        let mut chained = ogg(31);
        chained.extend(ogg_with_serial_and_rate(1, 2, 24_000));
        assert_eq!(ogg_duration_seconds(&chained), Some(32.0));
    }

    #[test]
    fn malformed_chained_streams_do_not_reduce_duration() {
        let mut reused_serial = ogg(31);
        reused_serial.extend(ogg_with_serial_and_rate(1, 1, 48_000));
        assert_eq!(ogg_duration_seconds(&reused_serial), None);

        let mut page_after_eos = ogg(31);
        let mut trailing_page = b"OggS\0\0".to_vec();
        trailing_page.extend_from_slice(&u64::MAX.to_le_bytes());
        trailing_page.extend_from_slice(&1u32.to_le_bytes());
        trailing_page.extend_from_slice(&2u32.to_le_bytes());
        trailing_page.extend_from_slice(&[0; 5]);
        page_after_eos.extend(trailing_page);
        assert_eq!(ogg_duration_seconds(&page_after_eos), None);

        let mut decreasing_granule = ogg(31);
        let mut final_page = b"OggS\0\x04".to_vec();
        final_page.extend_from_slice(&0u64.to_le_bytes());
        final_page.extend_from_slice(&1u32.to_le_bytes());
        final_page.extend_from_slice(&2u32.to_le_bytes());
        final_page.extend_from_slice(&[0; 5]);
        decreasing_granule.extend(final_page);
        assert_eq!(ogg_duration_seconds(&decreasing_granule), None);
    }
}
