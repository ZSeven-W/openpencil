//! `.fig` binary container layer — ports `fig-parser.ts::figToBinaryParts`.
//! Splits a `.fig` file into its decompressed binary parts (part 0 =
//! Kiwi schema, part 1 = Kiwi-encoded data) and extracts any embedded
//! images. Handles both the bare `fig-kiwi` container and the ZIP
//! archive wrapper (`canvas.fig` + `images/*`).

use crate::zip_reader::{read_zip, ZipError};
use std::collections::HashMap;
use std::io::Read;

/// 8-byte ASCII magic that prefixes the bare `fig-kiwi` container.
pub const FIG_KIWI_MAGIC: &[u8; 8] = b"fig-kiwi";
/// Zstandard frame magic (`0x28 0xB5 0x2F 0xFD`, little-endian u32).
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

/// Per-chunk decompressed ceiling — zip-bomb defence (1 GiB).
const MAX_CHUNK_SIZE: usize = 1024 * 1024 * 1024;

/// Result of splitting a `.fig` container.
pub struct FigBinaryParts {
    /// Decompressed binary parts in container order.
    pub parts: Vec<Vec<u8>>,
    /// Embedded image files keyed by the archive path minus `images/`.
    pub image_files: HashMap<String, Vec<u8>>,
}

#[derive(Debug)]
pub enum ContainerError {
    /// Input is neither a `fig-kiwi` container nor a ZIP holding one.
    NotFigContainer,
    /// ZIP wrapper present but `canvas.fig` is missing.
    NoCanvasFig,
    Zip(ZipError),
}

impl std::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerError::NotFigContainer => {
                write!(f, "not a .fig file: missing fig-kiwi header")
            }
            ContainerError::NoCanvasFig => {
                write!(f, "invalid .fig archive: no canvas.fig inside")
            }
            ContainerError::Zip(e) => write!(f, "{e}"),
        }
    }
}

fn has_fig_kiwi_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && &bytes[0..8] == FIG_KIWI_MAGIC
}

fn is_zstd(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0..4] == ZSTD_MAGIC
}

fn is_png(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 137 && bytes[1] == 80
}

fn u32_le(b: &[u8], off: usize) -> Option<u32> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Decompress one container chunk. Figma uses raw-deflate for the
/// schema chunk and may use zstd for the data chunk; PNG payloads are
/// passed through untouched. Mirrors `decompressChunk` exactly,
/// including the deflate→zstd→raw fallback chain.
fn decompress_chunk(bytes: &[u8]) -> Vec<u8> {
    if is_png(bytes) {
        return bytes.to_vec();
    }
    if is_zstd(bytes) {
        if let Some(out) = zstd_decompress(bytes) {
            return out;
        }
    }
    // Try raw deflate.
    let mut out = Vec::new();
    if flate2::read::DeflateDecoder::new(bytes)
        .take(MAX_CHUNK_SIZE as u64 + 1)
        .read_to_end(&mut out)
        .is_ok()
        && out.len() <= MAX_CHUNK_SIZE
    {
        return out;
    }
    // Deflate failed — fall back to zstd, then to the raw bytes.
    if let Some(out) = zstd_decompress(bytes) {
        return out;
    }
    bytes.to_vec()
}

fn zstd_decompress(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = ruzstd::StreamingDecoder::new(bytes).ok()?;
    let mut out = Vec::new();
    decoder
        .by_ref()
        .take(MAX_CHUNK_SIZE as u64 + 1)
        .read_to_end(&mut out)
        .ok()?;
    if out.len() > MAX_CHUNK_SIZE {
        return None;
    }
    Some(out)
}

/// Split a `.fig` file buffer into schema + data binary parts and
/// extract embedded images.
pub fn fig_to_binary_parts(file_buffer: &[u8]) -> Result<FigBinaryParts, ContainerError> {
    let mut image_files: HashMap<String, Vec<u8>> = HashMap::new();

    // ZIP wrapper: anything not starting with the fig-kiwi magic.
    let canvas: Vec<u8> = if has_fig_kiwi_magic(file_buffer) {
        file_buffer.to_vec()
    } else {
        let entries = read_zip(file_buffer).map_err(|e| match e {
            ZipError::NotZip => ContainerError::NotFigContainer,
            other => ContainerError::Zip(other),
        })?;
        let mut canvas_fig: Option<Vec<u8>> = None;
        for entry in entries {
            if let Some(key) = entry.name.strip_prefix("images/") {
                if !entry.data.is_empty() {
                    image_files.insert(key.to_string(), entry.data);
                }
            } else if entry.name == "canvas.fig" {
                canvas_fig = Some(entry.data);
            }
        }
        canvas_fig.ok_or(ContainerError::NoCanvasFig)?
    };

    if !has_fig_kiwi_magic(&canvas) {
        return Err(ContainerError::NotFigContainer);
    }

    // Skip 8-byte magic + 4-byte delimiter, then read length-prefixed
    // chunks until the buffer is exhausted.
    let mut start = 12usize;
    let mut parts: Vec<Vec<u8>> = Vec::new();
    while start < canvas.len() {
        let chunk_size = match u32_le(&canvas, start) {
            Some(s) => s as usize,
            None => break,
        };
        start += 4;
        if chunk_size == 0 || start + chunk_size > canvas.len() {
            break;
        }
        let raw_chunk = &canvas[start..start + chunk_size];
        parts.push(decompress_chunk(raw_chunk));
        start += chunk_size;
    }

    Ok(FigBinaryParts { parts, image_files })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Assemble a bare `fig-kiwi` container from raw (uncompressed)
    /// chunk payloads. Chunks are stored deflate-compressed so the
    /// decompressor round-trips them.
    fn make_fig_container(chunks: &[&[u8]]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(FIG_KIWI_MAGIC);
        buf.extend_from_slice(&[0, 0, 0, 0]); // 4-byte delimiter
        for chunk in chunks {
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(chunk).unwrap();
            let comp = enc.finish().unwrap();
            buf.extend_from_slice(&(comp.len() as u32).to_le_bytes());
            buf.extend_from_slice(&comp);
        }
        buf
    }

    #[test]
    fn splits_bare_container_into_parts() {
        let fig = make_fig_container(&[b"SCHEMA-BYTES", b"DATA-BYTES"]);
        let result = fig_to_binary_parts(&fig).expect("container parses");
        assert_eq!(result.parts.len(), 2);
        assert_eq!(result.parts[0], b"SCHEMA-BYTES");
        assert_eq!(result.parts[1], b"DATA-BYTES");
        assert!(result.image_files.is_empty());
    }

    #[test]
    fn deflate_chunk_round_trips() {
        // Chunks are deflate-compressed here (no zstd encoder in the
        // dev-deps); the zstd decode path is covered by `ruzstd`'s own
        // test suite and the magic-detection branch in decompress_chunk.
        let fig = make_fig_container(&[b"only-one"]);
        let result = fig_to_binary_parts(&fig).expect("parses");
        assert_eq!(result.parts.len(), 1);
        assert_eq!(result.parts[0], b"only-one");
    }

    #[test]
    fn rejects_non_container() {
        assert!(matches!(
            fig_to_binary_parts(b"random bytes here not a fig"),
            Err(ContainerError::NotFigContainer)
        ));
    }

    #[test]
    fn zip_wrapper_extracts_canvas_and_images() {
        use crate::zip_reader;
        let _ = zip_reader::read_zip; // keep the import meaningful
                                      // Build a ZIP holding canvas.fig (a real fig container) + an image.
        let inner = make_fig_container(&[b"S", b"D"]);
        let zip = make_zip_with(&[("canvas.fig", &inner), ("images/hash1", b"\x89PNGdata")]);
        let result = fig_to_binary_parts(&zip).expect("zip wrapper parses");
        assert_eq!(result.parts.len(), 2);
        assert_eq!(
            result.image_files.get("hash1").map(|v| v.as_slice()),
            Some(&b"\x89PNGdata"[..])
        );
    }

    /// Minimal multi-entry stored ZIP builder for tests.
    fn make_zip_with(files: &[(&str, &[u8])]) -> Vec<u8> {
        const LFH: u32 = 0x0403_4b50;
        const CDFH: u32 = 0x0201_4b50;
        const EOCD: u32 = 0x0605_4b50;
        let mut z = Vec::new();
        let mut cd = Vec::new();
        for (name, data) in files {
            let nb = name.as_bytes();
            let local_off = z.len() as u32;
            z.extend_from_slice(&LFH.to_le_bytes());
            z.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            z.extend_from_slice(&(data.len() as u32).to_le_bytes());
            z.extend_from_slice(&(data.len() as u32).to_le_bytes());
            z.extend_from_slice(&(nb.len() as u16).to_le_bytes());
            z.extend_from_slice(&[0, 0]);
            z.extend_from_slice(nb);
            z.extend_from_slice(data);
            cd.extend_from_slice(&CDFH.to_le_bytes());
            cd.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(nb.len() as u16).to_le_bytes());
            cd.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            cd.extend_from_slice(&local_off.to_le_bytes());
            cd.extend_from_slice(nb);
        }
        let cd_off = z.len() as u32;
        let cd_size = cd.len() as u32;
        z.extend_from_slice(&cd);
        z.extend_from_slice(&EOCD.to_le_bytes());
        z.extend_from_slice(&[0, 0, 0, 0]);
        z.extend_from_slice(&(files.len() as u16).to_le_bytes());
        z.extend_from_slice(&(files.len() as u16).to_le_bytes());
        z.extend_from_slice(&cd_size.to_le_bytes());
        z.extend_from_slice(&cd_off.to_le_bytes());
        z.extend_from_slice(&[0, 0]);
        z
    }
}
