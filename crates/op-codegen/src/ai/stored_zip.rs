//! Minimal STORED-only `.zip` encoder — pure std, zero deps.
//!
//! The web host downloads the AI structure bundle as a `.zip`
//! (TS `structure-bundle.ts` encodes with `uzip`); the desktop host
//! uses the `zip` crate, but pulling that crate into the wasm build
//! would cost bundle bytes against the 1 MiB gzip ceiling for a
//! single write-only, no-compression use. Entries are STORED
//! (method 0) — the bundle is JSON + already-compressed images, and
//! the artifact is gzipped by the transport anyway.
//!
//! Layout per APPNOTE.TXT: [local header + data]* then the central
//! directory and the end-of-central-directory record. No zip64 —
//! bundles are far below the 4 GiB / 65535-entry thresholds.

/// IEEE CRC-32 (reflected, poly 0xEDB88320) — the zip checksum.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Encode `files` as a STORED zip archive. Entry order is preserved.
pub fn build_stored_zip(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    // (name, crc, size, local-header offset) for the central directory.
    let mut entries: Vec<(&str, u32, u32, u32)> = Vec::with_capacity(files.len());

    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        push_u32(&mut out, 0x0403_4B50); // local file header signature
        push_u16(&mut out, 20); // version needed: 2.0
        push_u16(&mut out, 0); // flags
        push_u16(&mut out, 0); // method: STORED
        push_u16(&mut out, 0); // mod time
        push_u16(&mut out, 0); // mod date
        push_u32(&mut out, crc);
        push_u32(&mut out, size); // compressed size == size (STORED)
        push_u32(&mut out, size);
        push_u16(&mut out, name.len() as u16);
        push_u16(&mut out, 0); // extra len
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        entries.push((name, crc, size, offset));
    }

    let cd_offset = out.len() as u32;
    for (name, crc, size, offset) in &entries {
        push_u32(&mut out, 0x0201_4B50); // central directory signature
        push_u16(&mut out, 20); // version made by
        push_u16(&mut out, 20); // version needed
        push_u16(&mut out, 0); // flags
        push_u16(&mut out, 0); // method
        push_u16(&mut out, 0); // mod time
        push_u16(&mut out, 0); // mod date
        push_u32(&mut out, *crc);
        push_u32(&mut out, *size);
        push_u32(&mut out, *size);
        push_u16(&mut out, name.len() as u16);
        push_u16(&mut out, 0); // extra len
        push_u16(&mut out, 0); // comment len
        push_u16(&mut out, 0); // disk number
        push_u16(&mut out, 0); // internal attrs
        push_u32(&mut out, 0); // external attrs
        push_u32(&mut out, *offset);
        out.extend_from_slice(name.as_bytes());
    }
    let cd_size = out.len() as u32 - cd_offset;

    push_u32(&mut out, 0x0605_4B50); // end of central directory
    push_u16(&mut out, 0); // disk number
    push_u16(&mut out, 0); // cd start disk
    push_u16(&mut out, entries.len() as u16);
    push_u16(&mut out, entries.len() as u16);
    push_u32(&mut out, cd_size);
    push_u32(&mut out, cd_offset);
    push_u16(&mut out, 0); // comment len
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_known_vectors() {
        // Reference values from the IEEE CRC-32 ("123456789" check).
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn round_trips_through_the_zip_crate() {
        let files = vec![
            ("manifest.json".to_string(), br#"{"k":1}"#.to_vec()),
            ("assets/a.png".to_string(), vec![0x89, 0x50, 0x4E, 0x47]),
            ("views/raw.json".to_string(), b"[]".to_vec()),
        ];
        let bytes = build_stored_zip(&files);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("valid zip");
        assert_eq!(archive.len(), 3);
        for (name, data) in &files {
            use std::io::Read;
            let mut entry = archive.by_name(name).expect("entry present");
            let mut got = Vec::new();
            entry.read_to_end(&mut got).expect("read");
            assert_eq!(&got, data, "{name} round-trips");
        }
    }

    #[test]
    fn empty_archive_is_a_bare_eocd() {
        let bytes = build_stored_zip(&[]);
        assert_eq!(bytes.len(), 22, "EOCD record only");
        assert_eq!(&bytes[0..4], &[0x50, 0x4B, 0x05, 0x06]);
    }
}
