use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Seek, SeekFrom};

use zip::{CompressionMethod, ZipArchive};

use crate::html_encoding::decode_html_bytes;
use crate::project::{
    import_html_entry_with_fetcher, is_html_path, is_ignored_metadata_path,
    normalize_relative_path, select_html_entry_index, strip_common_root_paths,
    virtual_resource_key, virtual_url, HtmlProjectError, HtmlProjectFile, MAX_PROJECT_FILES,
};
use crate::resources::ImageTransform;
use crate::{wrap_imported_document, HtmlDocumentResult, HtmlImportOptions, HtmlImportResult};

pub const MAX_ZIP_ENTRIES: usize = 32_768;
pub(crate) const EOCD_MIN_BYTES: usize = 22;
const MAX_ZIP_COMMENT_BYTES: usize = u16::MAX as usize;

/// Fields of one end-of-central-directory record found in an archive tail.
/// Shared with the range-based reader in `project_zip_range`.
pub(crate) struct ZipEocdRecord {
    pub(crate) local_offset: usize,
    pub(crate) disk: u16,
    pub(crate) directory_disk: u16,
    pub(crate) disk_entries: u16,
    pub(crate) total_entries: u16,
    pub(crate) directory_size: u32,
    pub(crate) directory_offset: u32,
}

/// Scans `tail` backwards for the EOCD signature whose comment length reaches
/// exactly the end of the slice, then reads its fixed fields.
pub(crate) fn find_zip_eocd(tail: &[u8]) -> Option<ZipEocdRecord> {
    let last = tail.len().checked_sub(EOCD_MIN_BYTES)?;
    let local_offset = (0..=last).rev().find(|offset| {
        tail[*offset..].starts_with(b"PK\x05\x06")
            && read_u16(tail, *offset + 20)
                .is_some_and(|length| *offset + EOCD_MIN_BYTES + length as usize == tail.len())
    })?;
    Some(ZipEocdRecord {
        local_offset,
        disk: read_u16(tail, local_offset + 4)?,
        directory_disk: read_u16(tail, local_offset + 6)?,
        disk_entries: read_u16(tail, local_offset + 8)?,
        total_entries: read_u16(tail, local_offset + 10)?,
        directory_size: read_u32(tail, local_offset + 12)?,
        directory_offset: read_u32(tail, local_offset + 16)?,
    })
}

struct ZipManifestEntry {
    relative_path: String,
    archive_index: usize,
}

struct ZipManifest {
    entries: Vec<ZipManifestEntry>,
    html_entry: usize,
    html_entry_count: usize,
}

pub fn extract_html_zip(bytes: &[u8]) -> Result<Vec<HtmlProjectFile>, HtmlProjectError> {
    extract_html_zip_reader(Cursor::new(bytes))
}

pub fn extract_html_zip_reader<R: Read + Seek>(
    reader: R,
) -> Result<Vec<HtmlProjectFile>, HtmlProjectError> {
    let mut archive = open_archive(reader)?;
    let manifest = inspect_archive(&mut archive)?;
    let mut files = Vec::with_capacity(manifest.entries.len());
    for entry in manifest.entries {
        let bytes = read_entry(&mut archive, entry.archive_index)?;
        files.push(HtmlProjectFile::new(entry.relative_path, bytes));
    }
    Ok(files)
}

pub fn import_html_zip(
    bytes: &[u8],
    options: &HtmlImportOptions,
) -> Result<HtmlImportResult, HtmlProjectError> {
    import_html_zip_reader(Cursor::new(bytes), options)
}

pub fn import_html_zip_with_transform(
    bytes: &[u8],
    options: &HtmlImportOptions,
    transform: Option<&ImageTransform<'_>>,
) -> Result<HtmlImportResult, HtmlProjectError> {
    import_html_zip_reader_with_transform(Cursor::new(bytes), options, transform)
}

pub fn import_html_zip_document(
    bytes: &[u8],
    options: &HtmlImportOptions,
) -> Result<HtmlDocumentResult, HtmlProjectError> {
    import_html_zip_document_reader(Cursor::new(bytes), options)
}

pub fn import_html_zip_document_with_transform(
    bytes: &[u8],
    options: &HtmlImportOptions,
    transform: Option<&ImageTransform<'_>>,
) -> Result<HtmlDocumentResult, HtmlProjectError> {
    import_html_zip_document_reader_with_transform(Cursor::new(bytes), options, transform)
}

pub fn import_html_zip_reader<R: Read + Seek>(
    reader: R,
    options: &HtmlImportOptions,
) -> Result<HtmlImportResult, HtmlProjectError> {
    import_html_zip_reader_with_transform(reader, options, None)
}

pub fn import_html_zip_reader_with_transform<R: Read + Seek>(
    reader: R,
    options: &HtmlImportOptions,
    transform: Option<&ImageTransform<'_>>,
) -> Result<HtmlImportResult, HtmlProjectError> {
    let mut archive = open_archive(reader)?;
    let manifest = inspect_archive(&mut archive)?;
    let entry = &manifest.entries[manifest.html_entry];
    let entry_path = entry.relative_path.clone();
    let html_bytes = read_entry(&mut archive, entry.archive_index)?;
    let html = decode_html_bytes(&html_bytes);

    let resources: HashMap<String, usize> = manifest
        .entries
        .iter()
        .map(|entry| (virtual_url(&entry.relative_path), entry.archive_index))
        .collect();
    let archive = RefCell::new(archive);
    let fetcher = |requested: &str| {
        let key = virtual_resource_key(requested)?;
        let index = *resources.get(&key)?;
        read_entry(&mut archive.borrow_mut(), index).ok()
    };
    Ok(import_html_entry_with_fetcher(
        &html,
        &entry_path,
        manifest.html_entry_count,
        options,
        &fetcher,
        transform,
    ))
}

pub fn import_html_zip_document_reader<R: Read + Seek>(
    reader: R,
    options: &HtmlImportOptions,
) -> Result<HtmlDocumentResult, HtmlProjectError> {
    import_html_zip_document_reader_with_transform(reader, options, None)
}

pub fn import_html_zip_document_reader_with_transform<R: Read + Seek>(
    reader: R,
    options: &HtmlImportOptions,
    transform: Option<&ImageTransform<'_>>,
) -> Result<HtmlDocumentResult, HtmlProjectError> {
    import_html_zip_reader_with_transform(reader, options, transform).map(wrap_imported_document)
}

pub fn import_html_zip_reader_document<R: Read + Seek>(
    reader: R,
    options: &HtmlImportOptions,
) -> Result<HtmlDocumentResult, HtmlProjectError> {
    import_html_zip_document_reader(reader, options)
}

pub fn import_html_zip_reader_document_with_transform<R: Read + Seek>(
    reader: R,
    options: &HtmlImportOptions,
    transform: Option<&ImageTransform<'_>>,
) -> Result<HtmlDocumentResult, HtmlProjectError> {
    import_html_zip_document_reader_with_transform(reader, options, transform)
}

fn open_archive<R: Read + Seek>(mut reader: R) -> Result<ZipArchive<R>, HtmlProjectError> {
    preflight_zip_directory(&mut reader)?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|error| HtmlProjectError::InvalidZip(error.to_string()))?;
    ZipArchive::new(reader).map_err(|error| HtmlProjectError::InvalidZip(error.to_string()))
}

fn inspect_archive<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<ZipManifest, HtmlProjectError> {
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(HtmlProjectError::TooManyFiles {
            count: archive.len(),
            limit: MAX_ZIP_ENTRIES,
        });
    }
    let mut entries = Vec::with_capacity(archive.len().min(MAX_PROJECT_FILES));
    let mut seen = HashSet::new();
    for archive_index in 0..archive.len() {
        let metadata = archive
            .by_index_raw(archive_index)
            .map_err(|error| HtmlProjectError::InvalidZip(error.to_string()))?;
        let original_path = metadata
            .name()
            .map_err(|error| HtmlProjectError::InvalidZip(error.to_string()))?
            .into_owned();
        let is_symlink = metadata.is_symlink();
        let is_dir = metadata.is_dir();
        let encrypted = metadata.encrypted();
        let compression = metadata.compression();
        let unix_mode = metadata.unix_mode();
        drop(metadata);

        let path = normalize_relative_path(&original_path)?;
        if is_symlink {
            return Err(HtmlProjectError::UnsupportedZipEntry {
                path,
                reason: "symbolic links are not imported".to_string(),
            });
        }
        if is_dir || is_ignored_metadata_path(&path) {
            continue;
        }
        if entries.len() >= MAX_PROJECT_FILES {
            return Err(HtmlProjectError::TooManyFiles {
                count: entries.len() + 1,
                limit: MAX_PROJECT_FILES,
            });
        }
        if unix_mode.is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000) {
            return Err(HtmlProjectError::UnsupportedZipEntry {
                path,
                reason: "only regular files are imported".to_string(),
            });
        }
        if encrypted {
            return Err(HtmlProjectError::UnsupportedZipEntry {
                path,
                reason: "encrypted entries are not supported".to_string(),
            });
        }
        if !matches!(
            compression,
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(HtmlProjectError::UnsupportedZipEntry {
                path,
                reason: format!(
                    "compression method {compression:?} is not supported; use stored or deflate"
                ),
            });
        }
        if !seen.insert(path.clone()) {
            return Err(HtmlProjectError::DuplicatePath(path));
        }
        entries.push(ZipManifestEntry {
            relative_path: path,
            archive_index,
        });
    }
    if entries.is_empty() {
        return Err(HtmlProjectError::EmptyProject);
    }
    let mut paths: Vec<String> = entries
        .iter()
        .map(|entry| entry.relative_path.clone())
        .collect();
    strip_common_root_paths(&mut paths);
    let mut stripped_seen = HashSet::new();
    for (entry, path) in entries.iter_mut().zip(paths) {
        if !stripped_seen.insert(path.clone()) {
            return Err(HtmlProjectError::DuplicatePath(path));
        }
        entry.relative_path = path;
    }
    let paths: Vec<String> = entries
        .iter()
        .map(|entry| entry.relative_path.clone())
        .collect();
    let html_entry = select_html_entry_index(&paths)?;
    let html_entry_count = paths.iter().filter(|path| is_html_path(path)).count();
    Ok(ZipManifest {
        entries,
        html_entry,
        html_entry_count,
    })
}

fn read_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    index: usize,
) -> Result<Vec<u8>, HtmlProjectError> {
    let mut entry = archive
        .by_index(index)
        .map_err(|error| HtmlProjectError::InvalidZip(error.to_string()))?;
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|error| HtmlProjectError::InvalidZip(error.to_string()))?;
    Ok(bytes)
}

fn preflight_zip_directory<R: Read + Seek>(reader: &mut R) -> Result<(), HtmlProjectError> {
    let archive_len = reader
        .seek(SeekFrom::End(0))
        .map_err(|error| HtmlProjectError::InvalidZip(error.to_string()))?;
    if archive_len < EOCD_MIN_BYTES as u64 {
        return Err(HtmlProjectError::InvalidZip(
            "end-of-central-directory record is missing".to_string(),
        ));
    }
    let tail_len = archive_len.min((EOCD_MIN_BYTES + MAX_ZIP_COMMENT_BYTES) as u64) as usize;
    reader
        .seek(SeekFrom::End(-(tail_len as i64)))
        .map_err(|error| HtmlProjectError::InvalidZip(error.to_string()))?;
    let mut tail = vec![0; tail_len];
    reader
        .read_exact(&mut tail)
        .map_err(|error| HtmlProjectError::InvalidZip(error.to_string()))?;
    let Some(eocd) = find_zip_eocd(&tail) else {
        return Err(HtmlProjectError::InvalidZip(
            "end-of-central-directory record is missing or malformed".to_string(),
        ));
    };
    if eocd.disk != 0 || eocd.directory_disk != 0 || eocd.disk_entries != eocd.total_entries {
        return Err(HtmlProjectError::InvalidZip(
            "multi-disk ZIP archives are not supported".to_string(),
        ));
    }
    if eocd.total_entries == u16::MAX
        || eocd.directory_size == u32::MAX
        || eocd.directory_offset == u32::MAX
    {
        return Err(HtmlProjectError::InvalidZip(
            "ZIP64 archives are not supported for HTML import".to_string(),
        ));
    }
    let eocd_offset = archive_len - tail_len as u64 + eocd.local_offset as u64;
    if (eocd.directory_offset as u64)
        .checked_add(eocd.directory_size as u64)
        .is_none_or(|end| end > eocd_offset)
    {
        return Err(HtmlProjectError::InvalidZip(
            "central-directory bounds are invalid".to_string(),
        ));
    }
    if eocd.total_entries as usize > MAX_ZIP_ENTRIES {
        return Err(HtmlProjectError::TooManyFiles {
            count: eocd.total_entries as usize,
            limit: MAX_ZIP_ENTRIES,
        });
    }
    Ok(())
}

pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

/// Builds an in-memory ZIP archive for tests; shared with `project_zip_range`.
#[cfg(test)]
pub(crate) fn test_archive(entries: &[(&str, &[u8], CompressionMethod)]) -> Vec<u8> {
    use std::io::Write;

    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes, method) in entries {
        writer
            .start_file(
                *path,
                zip::write::SimpleFileOptions::default().compression_method(*method),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::project_zip::test_archive as archive;

    fn central_offsets(bytes: &[u8]) -> Vec<usize> {
        bytes
            .windows(4)
            .enumerate()
            .filter_map(|(offset, window)| (window == b"PK\x01\x02").then_some(offset))
            .collect()
    }

    #[test]
    fn imports_stored_and_deflated_resources_lazily_from_a_reader() {
        let bytes = archive(&[
            (
                "site/index.html",
                br#"<link rel="stylesheet" href="/assets/site.css"><img src="/assets/p.png">"#,
                CompressionMethod::Deflated,
            ),
            (
                "site/assets/site.css",
                b"body{background-image:url('/assets/p.png')}",
                CompressionMethod::Stored,
            ),
            (
                "site/assets/p.png",
                &[0x89, 0x50, 0x4e, 0x47, 1],
                CompressionMethod::Deflated,
            ),
        ]);
        let imported =
            import_html_zip_reader(Cursor::new(bytes), &HtmlImportOptions::default()).unwrap();
        let json = serde_json::to_string(&imported.nodes).unwrap();
        assert!(
            json.matches("data:image/png;base64,").count() >= 2,
            "{json}"
        );
    }

    #[test]
    fn imports_a_non_utf8_html_entry() {
        let mut html = b"<meta charset=gbk><p>".to_vec();
        html.extend_from_slice(b"\xC4\xE3\xBA\xC3");
        html.extend_from_slice(b"</p>");
        let bytes = archive(&[("index.html", &html, CompressionMethod::Deflated)]);
        let imported = import_html_zip(&bytes, &HtmlImportOptions::default()).unwrap();
        let json = serde_json::to_string(&imported.nodes).unwrap();
        assert!(json.contains("你好"), "{json}");
    }

    #[test]
    fn unreferenced_corrupt_resource_with_huge_declared_size_is_not_decompressed() {
        let unused = b"unused-corrupt-payload";
        let mut bytes = archive(&[
            ("site/index.html", b"<p>ok</p>", CompressionMethod::Stored),
            ("site/unused.bin", unused, CompressionMethod::Stored),
        ]);
        let payload = bytes
            .windows(unused.len())
            .position(|window| window == unused)
            .unwrap();
        bytes[payload] ^= 0xff;
        let offsets = central_offsets(&bytes);
        bytes[offsets[1] + 24..offsets[1] + 28].copy_from_slice(&500_000_000u32.to_le_bytes());
        let imported = import_html_zip(&bytes, &HtmlImportOptions::default()).unwrap();
        assert_eq!(imported.nodes.len(), 1);
        assert!(extract_html_zip(&bytes).is_err());
    }

    #[test]
    fn rejects_an_unreferenced_entry_with_a_malformed_local_header() {
        let mut bytes = archive(&[
            ("index.html", b"<p>ok</p>", CompressionMethod::Stored),
            ("unused.bin", b"unused", CompressionMethod::Stored),
        ]);
        let central = central_offsets(&bytes)[1];
        let local = read_u32(&bytes, central + 42).unwrap() as usize;
        bytes[local..local + 4].fill(0);
        assert!(matches!(
            import_html_zip(&bytes, &HtmlImportOptions::default()),
            Err(HtmlProjectError::InvalidZip(_))
        ));
    }

    #[test]
    fn rejects_traversal_duplicate_encrypted_and_non_regular_entries() {
        let traversal = archive(&[("../index.html", b"x", CompressionMethod::Stored)]);
        assert!(matches!(
            import_html_zip(&traversal, &HtmlImportOptions::default()),
            Err(HtmlProjectError::InvalidPath { .. })
        ));
        let duplicate = archive(&[
            ("index.html", b"a", CompressionMethod::Stored),
            ("./index.html", b"b", CompressionMethod::Stored),
        ]);
        assert!(matches!(
            import_html_zip(&duplicate, &HtmlImportOptions::default()),
            Err(HtmlProjectError::DuplicatePath(_))
        ));

        let mut encrypted = archive(&[("index.html", b"x", CompressionMethod::Stored)]);
        encrypted[6] |= 1;
        let central = central_offsets(&encrypted)[0];
        encrypted[central + 8] |= 1;
        assert!(matches!(
            import_html_zip(&encrypted, &HtmlImportOptions::default()),
            Err(HtmlProjectError::UnsupportedZipEntry { reason, .. })
                if reason.contains("encrypted")
        ));

        let mut fifo = archive(&[("index.html", b"x", CompressionMethod::Stored)]);
        let central = central_offsets(&fifo)[0];
        fifo[central + 5] = 3;
        fifo[central + 38..central + 42].copy_from_slice(&(0o010644u32 << 16).to_le_bytes());
        assert!(matches!(
            import_html_zip(&fifo, &HtmlImportOptions::default()),
            Err(HtmlProjectError::UnsupportedZipEntry { reason, .. })
                if reason.contains("regular")
        ));
    }

    #[test]
    fn rejects_no_html_truncated_zip64_multi_disk_and_excessive_entries() {
        let no_html = archive(&[("site.css", b"body{}", CompressionMethod::Stored)]);
        assert!(matches!(
            import_html_zip(&no_html, &HtmlImportOptions::default()),
            Err(HtmlProjectError::NoHtmlEntry)
        ));
        let mut truncated = archive(&[("index.html", b"x", CompressionMethod::Stored)]);
        truncated.pop();
        assert!(matches!(
            import_html_zip(&truncated, &HtmlImportOptions::default()),
            Err(HtmlProjectError::InvalidZip(_))
        ));

        let source = archive(&[("index.html", b"x", CompressionMethod::Stored)]);
        for (field, value) in [(8usize, u16::MAX), (10, u16::MAX), (4, 1)] {
            let mut patched = source.clone();
            let eocd = patched.len() - EOCD_MIN_BYTES;
            patched[eocd + field..eocd + field + 2].copy_from_slice(&value.to_le_bytes());
            assert!(import_html_zip(&patched, &HtmlImportOptions::default()).is_err());
        }

        let mut excessive = source;
        let eocd = excessive.len() - EOCD_MIN_BYTES;
        let count = (MAX_ZIP_ENTRIES as u16 + 1).to_le_bytes();
        excessive[eocd + 8..eocd + 10].copy_from_slice(&count);
        excessive[eocd + 10..eocd + 12].copy_from_slice(&count);
        assert!(matches!(
            import_html_zip(&excessive, &HtmlImportOptions::default()),
            Err(HtmlProjectError::TooManyFiles { .. })
        ));
    }
}
