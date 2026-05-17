//! Minimal ZIP archive reader — just enough to enumerate entries of a
//! `.fig` archive (Figma wraps `canvas.fig` + `images/*` in a plain
//! store/deflate ZIP). Hand-rolled to avoid pulling the full `zip`
//! crate's dependency tree; covers standard 32-bit ZIP with
//! store (method 0) and deflate (method 8) entries.

use std::io::Read;

/// One decompressed archive entry.
pub struct ZipEntry {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub enum ZipError {
    NotZip,
    Truncated,
    UnsupportedMethod(u16),
    Inflate(String),
    TooLarge,
}

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZipError::NotZip => write!(f, "not a ZIP archive (no end-of-central-directory record)"),
            ZipError::Truncated => write!(f, "ZIP archive is truncated"),
            ZipError::UnsupportedMethod(m) => write!(f, "unsupported ZIP compression method {m}"),
            ZipError::Inflate(e) => write!(f, "ZIP inflate failed: {e}"),
            ZipError::TooLarge => write!(f, "ZIP entry exceeds size limit"),
        }
    }
}

const EOCD_SIG: u32 = 0x0605_4b50;
const CDFH_SIG: u32 = 0x0201_4b50;
const LFH_SIG: u32 = 0x0403_4b50;
/// Per-entry decompressed ceiling — zip-bomb defence (512 MiB).
const MAX_ENTRY_SIZE: usize = 512 * 1024 * 1024;
/// Aggregate decompressed ceiling across the whole archive (2 GiB).
const MAX_TOTAL_SIZE: usize = 2 * 1024 * 1024 * 1024;
/// Cap on the number of archive entries (many-tiny-files zip bomb).
const MAX_ENTRIES: usize = 10_000;

fn u16_le(b: &[u8], off: usize) -> Option<u16> {
    b.get(off..off + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
}

fn u32_le(b: &[u8], off: usize) -> Option<u32> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Locate the End-of-Central-Directory record by scanning backwards
/// from the buffer tail (the EOCD's trailing comment is variable-length
/// so it cannot be addressed by a fixed offset).
fn find_eocd(buf: &[u8]) -> Option<usize> {
    if buf.len() < 22 {
        return None;
    }
    // EOCD is 22 bytes + up to 64 KiB comment.
    let scan_start = buf.len().saturating_sub(22 + 0xffff);
    for off in (scan_start..=buf.len() - 22).rev() {
        if u32_le(buf, off) == Some(EOCD_SIG) {
            return Some(off);
        }
    }
    None
}

/// Parse a ZIP archive into its decompressed entries. Entry order
/// follows the central directory.
pub fn read_zip(buf: &[u8]) -> Result<Vec<ZipEntry>, ZipError> {
    let eocd = find_eocd(buf).ok_or(ZipError::NotZip)?;
    let total_entries = u16_le(buf, eocd + 10).ok_or(ZipError::Truncated)? as usize;
    let cd_offset = u32_le(buf, eocd + 16).ok_or(ZipError::Truncated)? as usize;
    if total_entries > MAX_ENTRIES {
        return Err(ZipError::TooLarge);
    }

    let mut entries = Vec::with_capacity(total_entries);
    let mut total_size = 0usize;
    let mut cursor = cd_offset;
    for _ in 0..total_entries {
        if u32_le(buf, cursor) != Some(CDFH_SIG) {
            break;
        }
        let method = u16_le(buf, cursor + 10).ok_or(ZipError::Truncated)?;
        let comp_size = u32_le(buf, cursor + 20).ok_or(ZipError::Truncated)? as usize;
        let uncomp_size = u32_le(buf, cursor + 24).ok_or(ZipError::Truncated)? as usize;
        let name_len = u16_le(buf, cursor + 28).ok_or(ZipError::Truncated)? as usize;
        let extra_len = u16_le(buf, cursor + 30).ok_or(ZipError::Truncated)? as usize;
        let comment_len = u16_le(buf, cursor + 32).ok_or(ZipError::Truncated)? as usize;
        let local_off = u32_le(buf, cursor + 42).ok_or(ZipError::Truncated)? as usize;

        let name_bytes = buf
            .get(cursor + 46..cursor + 46 + name_len)
            .ok_or(ZipError::Truncated)?;
        let name = String::from_utf8_lossy(name_bytes).into_owned();

        // Aggregate-size guard runs on the central-directory's declared
        // size *before* the entry is decompressed.
        total_size = total_size.saturating_add(uncomp_size);
        if total_size > MAX_TOTAL_SIZE {
            return Err(ZipError::TooLarge);
        }
        let data = read_entry_data(buf, local_off, method, comp_size, uncomp_size)?;
        entries.push(ZipEntry { name, data });

        cursor += 46 + name_len + extra_len + comment_len;
    }
    Ok(entries)
}

/// Read + decompress one entry's payload, located via its local file
/// header (the central-directory sizes are authoritative — local
/// headers may carry zeroed sizes when a data descriptor is used).
fn read_entry_data(
    buf: &[u8],
    local_off: usize,
    method: u16,
    comp_size: usize,
    uncomp_size: usize,
) -> Result<Vec<u8>, ZipError> {
    if u32_le(buf, local_off) != Some(LFH_SIG) {
        return Err(ZipError::Truncated);
    }
    let lfh_name_len = u16_le(buf, local_off + 26).ok_or(ZipError::Truncated)? as usize;
    let lfh_extra_len = u16_le(buf, local_off + 28).ok_or(ZipError::Truncated)? as usize;
    let data_start = local_off + 30 + lfh_name_len + lfh_extra_len;
    let raw = buf
        .get(data_start..data_start + comp_size)
        .ok_or(ZipError::Truncated)?;

    match method {
        0 => {
            if comp_size > MAX_ENTRY_SIZE {
                return Err(ZipError::TooLarge);
            }
            Ok(raw.to_vec())
        }
        8 => {
            if uncomp_size > MAX_ENTRY_SIZE {
                return Err(ZipError::TooLarge);
            }
            let mut out = Vec::with_capacity(uncomp_size.min(MAX_ENTRY_SIZE));
            flate2::read::DeflateDecoder::new(raw)
                .take(MAX_ENTRY_SIZE as u64 + 1)
                .read_to_end(&mut out)
                .map_err(|e| ZipError::Inflate(e.to_string()))?;
            if out.len() > MAX_ENTRY_SIZE {
                return Err(ZipError::TooLarge);
            }
            Ok(out)
        }
        other => Err(ZipError::UnsupportedMethod(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Build a minimal single-entry ZIP with one stored (method 0) file.
    fn make_stored_zip(name: &str, data: &[u8]) -> Vec<u8> {
        let mut z = Vec::new();
        let name_b = name.as_bytes();
        // Local file header.
        let local_off = z.len() as u32;
        z.extend_from_slice(&LFH_SIG.to_le_bytes());
        z.extend_from_slice(&[20, 0]); // version needed
        z.extend_from_slice(&[0, 0]); // flags
        z.extend_from_slice(&[0, 0]); // method = store
        z.extend_from_slice(&[0, 0, 0, 0]); // time+date
        z.extend_from_slice(&[0, 0, 0, 0]); // crc32 (unchecked)
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        z.extend_from_slice(&[0, 0]); // extra len
        z.extend_from_slice(name_b);
        z.extend_from_slice(data);
        // Central directory.
        let cd_off = z.len() as u32;
        z.extend_from_slice(&CDFH_SIG.to_le_bytes());
        z.extend_from_slice(&[20, 0, 20, 0]); // version made/needed
        z.extend_from_slice(&[0, 0, 0, 0]); // flags+method
        z.extend_from_slice(&[0, 0, 0, 0]); // time+date
        z.extend_from_slice(&[0, 0, 0, 0]); // crc32
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        z.extend_from_slice(&[0, 0, 0, 0]); // extra+comment len
        z.extend_from_slice(&[0, 0, 0, 0]); // disk+internal attrs
        z.extend_from_slice(&[0, 0, 0, 0]); // external attrs
        z.extend_from_slice(&local_off.to_le_bytes());
        z.extend_from_slice(name_b);
        let cd_size = z.len() as u32 - cd_off;
        // EOCD.
        z.extend_from_slice(&EOCD_SIG.to_le_bytes());
        z.extend_from_slice(&[0, 0, 0, 0]); // disk numbers
        z.extend_from_slice(&[1, 0, 1, 0]); // entries on disk / total
        z.extend_from_slice(&cd_size.to_le_bytes());
        z.extend_from_slice(&cd_off.to_le_bytes());
        z.extend_from_slice(&[0, 0]); // comment len
        z
    }

    /// Build a single-entry ZIP whose payload is raw-deflate compressed.
    fn make_deflate_zip(name: &str, data: &[u8]) -> Vec<u8> {
        let mut enc =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(data).unwrap();
        let comp = enc.finish().unwrap();
        let mut z = Vec::new();
        let name_b = name.as_bytes();
        let local_off = z.len() as u32;
        z.extend_from_slice(&LFH_SIG.to_le_bytes());
        z.extend_from_slice(&[20, 0, 0, 0]);
        z.extend_from_slice(&[8, 0]); // method = deflate
        z.extend_from_slice(&[0, 0, 0, 0]);
        z.extend_from_slice(&[0, 0, 0, 0]);
        z.extend_from_slice(&(comp.len() as u32).to_le_bytes());
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        z.extend_from_slice(&[0, 0]);
        z.extend_from_slice(name_b);
        z.extend_from_slice(&comp);
        let cd_off = z.len() as u32;
        z.extend_from_slice(&CDFH_SIG.to_le_bytes());
        z.extend_from_slice(&[20, 0, 20, 0]);
        z.extend_from_slice(&[0, 0]);
        z.extend_from_slice(&[8, 0]); // method = deflate
        z.extend_from_slice(&[0, 0, 0, 0]);
        z.extend_from_slice(&[0, 0, 0, 0]);
        z.extend_from_slice(&(comp.len() as u32).to_le_bytes());
        z.extend_from_slice(&(data.len() as u32).to_le_bytes());
        z.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        z.extend_from_slice(&[0, 0, 0, 0]);
        z.extend_from_slice(&[0, 0, 0, 0]);
        z.extend_from_slice(&[0, 0, 0, 0]);
        z.extend_from_slice(&local_off.to_le_bytes());
        z.extend_from_slice(name_b);
        let cd_size = z.len() as u32 - cd_off;
        z.extend_from_slice(&EOCD_SIG.to_le_bytes());
        z.extend_from_slice(&[0, 0, 0, 0]);
        z.extend_from_slice(&[1, 0, 1, 0]);
        z.extend_from_slice(&cd_size.to_le_bytes());
        z.extend_from_slice(&cd_off.to_le_bytes());
        z.extend_from_slice(&[0, 0]);
        z
    }

    #[test]
    fn reads_stored_entry() {
        let zip = make_stored_zip("canvas.fig", b"hello fig");
        let entries = read_zip(&zip).expect("stored zip parses");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "canvas.fig");
        assert_eq!(entries[0].data, b"hello fig");
    }

    #[test]
    fn reads_deflate_entry() {
        let payload = b"the quick brown fox jumps over the lazy dog".repeat(20);
        let zip = make_deflate_zip("images/abc", &payload);
        let entries = read_zip(&zip).expect("deflate zip parses");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "images/abc");
        assert_eq!(entries[0].data, payload);
    }

    #[test]
    fn rejects_non_zip() {
        assert!(matches!(
            read_zip(b"not a zip at all"),
            Err(ZipError::NotZip)
        ));
    }
}
