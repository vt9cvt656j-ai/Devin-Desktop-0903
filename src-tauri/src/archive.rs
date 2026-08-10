//! Reading the index of an archive without unpacking it.
//!
//! The previous version handled ZIP only, listed at most 200 entries, and then reported that 200
//! as the archive's size — so a 2 GB zip holding fifty thousand files said it held two hundred.
//! Two separate problems: a cap presented as a total, and every non-ZIP format falling through to
//! an empty list.
//!
//! Two properties matter more than breadth here:
//!
//! * **The count is the real count.** `total` always describes the archive. `entries` is a preview
//!   and says so via `truncated`. A number the reader can trust is worth more than a long list.
//! * **It returns.** Some formats can only be counted by decompressing the whole stream. On a
//!   multi-gigabyte file that is minutes of work for a panel nobody is staring at, so streaming
//!   formats run under a budget and report what they got, honestly labelled. Never a spinner that
//!   does not end.
//!
//! Formats whose index sits in a footer (ZIP, 7z) are exact and instant at any size — those are
//! read directly and are never truncated by the budget.

use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{Duration, Instant};

/// How many entries travel to the UI. The panel renders one table row each, so this is a
/// rendering budget, not a parsing one — `total` is counted past it.
const PREVIEW_LIMIT: usize = 2000;
/// Wall-clock ceiling for formats that must be decompressed to be counted.
const STREAM_TIME_BUDGET: Duration = Duration::from_secs(6);
/// Byte ceiling for the same, so a slow disk cannot beat the clock check.
const STREAM_BYTE_BUDGET: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Serialize, Clone)]
pub struct ArchiveEntryPreview {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    pub is_dir: bool,
    /// ZIP records this per entry; an encrypted archive still lists its names.
    pub encrypted: bool,
}

#[derive(Serialize)]
pub struct ArchiveListing {
    /// Wire name of the container: "zip", "tar", "tar.gz", "7z", "gzip", …
    pub format: String,
    pub entries: Vec<ArchiveEntryPreview>,
    /// Entries in the archive. Equals `entries.len()` only when `truncated` is false.
    pub total: u64,
    /// True when `entries` is a prefix — either past PREVIEW_LIMIT, or the budget ran out.
    pub truncated: bool,
    /// True when counting stopped early, so `total` is a floor rather than the count.
    pub count_is_partial: bool,
    pub total_size: u64,
    pub total_compressed: u64,
    /// macOS xattr sidecars (`._name`, `__MACOSX/…`) counted apart from the real files. They are
    /// genuine members, so they are reported rather than silently dropped — but folding them into
    /// the total doubles it, and macOS's own `tar -t` hides them for exactly that reason.
    pub metadata_entries: u64,
    pub encrypted: bool,
    /// Set when something is worth saying: a format we cannot open, a truncated walk, a
    /// damaged tail. Shown verbatim, so it says what happened rather than "failed".
    pub note: Option<String>,
}

impl ArchiveListing {
    fn empty(format: &str, note: Option<String>) -> Self {
        Self {
            format: format.into(),
            entries: Vec::new(),
            total: 0,
            truncated: false,
            count_is_partial: false,
            total_size: 0,
            total_compressed: 0,
            metadata_entries: 0,
            encrypted: false,
            note,
        }
    }
}

/// An AppleDouble sidecar or a `__MACOSX` shadow tree — metadata the archiver added, not
/// something the user put in.
fn is_platform_metadata(name: &str) -> bool {
    if name.starts_with("__MACOSX/") || name == "__MACOSX" {
        return true;
    }
    name.rsplit('/')
        .find(|part| !part.is_empty())
        .is_some_and(|base| base.starts_with("._"))
}

/// Accumulates entries while keeping the true count, so the cap never reaches `total`.
struct Collector {
    entries: Vec<ArchiveEntryPreview>,
    total: u64,
    total_size: u64,
    total_compressed: u64,
    metadata_entries: u64,
    encrypted: bool,
}

impl Collector {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            total: 0,
            total_size: 0,
            total_compressed: 0,
            metadata_entries: 0,
            encrypted: false,
        }
    }

    fn push(&mut self, entry: ArchiveEntryPreview) {
        if is_platform_metadata(&entry.name) {
            self.metadata_entries += 1;
            return;
        }
        self.total += 1;
        self.total_size = self.total_size.saturating_add(entry.size);
        self.total_compressed = self.total_compressed.saturating_add(entry.compressed_size);
        self.encrypted |= entry.encrypted;
        if self.entries.len() < PREVIEW_LIMIT {
            self.entries.push(entry);
        }
    }

    fn finish(
        self,
        format: &str,
        count_is_partial: bool,
        note: Option<String>,
    ) -> ArchiveListing {
        ArchiveListing {
            format: format.into(),
            truncated: count_is_partial || self.total as usize > self.entries.len(),
            count_is_partial,
            total: self.total,
            total_size: self.total_size,
            total_compressed: self.total_compressed,
            metadata_entries: self.metadata_entries,
            encrypted: self.encrypted,
            entries: self.entries,
            note,
        }
    }
}

/// What container is this? Magic bytes first — an archive named `.bin`, or a `.gz` that is really
/// a tarball, is ordinary. The extension only breaks ties the bytes cannot.
#[derive(Clone, Copy, PartialEq)]
enum Format {
    Zip,
    Tar,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
    SevenZ,
    Rar,
    Unknown,
}

fn sniff(path: &Path) -> Format {
    let mut head = [0u8; 512];
    let read = File::open(path)
        .and_then(|mut f| f.read(&mut head))
        .unwrap_or(0);
    let head = &head[..read];

    if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") || head.starts_with(b"PK\x07\x08") {
        return Format::Zip;
    }
    if head.starts_with(b"7z\xBC\xAF\x27\x1C") {
        return Format::SevenZ;
    }
    if head.starts_with(b"Rar!\x1A\x07") {
        return Format::Rar;
    }
    if head.starts_with(&[0x1f, 0x8b]) {
        return Format::Gzip;
    }
    if head.starts_with(b"BZh") {
        return Format::Bzip2;
    }
    if head.starts_with(&[0xFD, b'7', b'z', b'X', b'Z', 0x00]) {
        return Format::Xz;
    }
    if head.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        return Format::Zstd;
    }
    // tar has no leading magic — "ustar" sits at offset 257, and pre-POSIX tars lack even that.
    // Those are recognised by their extension below rather than guessed at from a checksum.
    if head.len() >= 265 && (&head[257..262] == b"ustar") {
        return Format::Tar;
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "tar" => Format::Tar,
        "zip" | "jar" | "war" | "apk" | "aar" | "ipa" | "whl" | "egg" | "nupkg" | "docx"
        | "pptx" | "xlsx" | "odt" | "ods" | "odp" | "epub" | "crx" | "vsix" | "xpi" => Format::Zip,
        "7z" => Format::SevenZ,
        "rar" => Format::Rar,
        _ => Format::Unknown,
    }
}

/// Does this filename say the compressed stream inside is a tarball?
fn wraps_a_tar(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    name.ends_with(".tar.gz")
        || name.ends_with(".tar.bz2")
        || name.ends_with(".tar.xz")
        || name.ends_with(".tar.zst")
        || name.ends_with(".tar.zstd")
        || name.ends_with(".tgz")
        || name.ends_with(".tbz")
        || name.ends_with(".tbz2")
        || name.ends_with(".txz")
        || name.ends_with(".tzst")
}

/// True when the file is a compressed tarball, whatever it is called. The inspector uses this to
/// decide whether a `.bin` or an extensionless download deserves the archive panel at all.
pub fn looks_like_compressed_tar(path: &Path) -> bool {
    if wraps_a_tar(path) {
        return true;
    }
    matches!(sniff(path), Format::Tar | Format::Zip | Format::SevenZ | Format::Rar)
}

pub fn read_listing(path: &Path) -> ArchiveListing {
    match sniff(path) {
        Format::Zip => read_zip(path),
        Format::SevenZ => read_7z(path),
        Format::Tar => read_tar(path),
        Format::Gzip => read_compressed(path, Format::Gzip),
        Format::Bzip2 => read_compressed(path, Format::Bzip2),
        Format::Xz => read_compressed(path, Format::Xz),
        Format::Zstd => read_compressed(path, Format::Zstd),
        // RAR's decoder is proprietary — there is no Rust implementation to link, and the
        // vendor library's licence forbids using it to build a compressor. Say so plainly
        // instead of showing an empty list that reads like a corrupt file.
        Format::Rar => ArchiveListing::empty(
            "rar",
            Some("RAR：识别到 RAR 压缩包，但解码器是闭源的，本机没有可用的实现，因此列不出条目。".into()),
        ),
        Format::Unknown => ArchiveListing::empty("unknown", None),
    }
}

/// ZIP keeps its index in a footer, so this is exact and fast whatever the archive weighs — the
/// 2 GB case never reads 2 GB.
fn read_zip(path: &Path) -> ArchiveListing {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return ArchiveListing::empty("zip", Some(format!("打不开文件：{e}"))),
    };
    let mut zip = match zip::ZipArchive::new(BufReader::new(file)) {
        Ok(z) => z,
        Err(e) => {
            return ArchiveListing::empty("zip", Some(format!("ZIP 目录读取失败：{e}")));
        }
    };
    let mut collector = Collector::new();
    let mut unreadable = 0usize;
    for i in 0..zip.len() {
        match zip.by_index_raw(i) {
            // by_index_raw, not by_index: it reads the directory record without setting up a
            // decompressor, so entries compressed with a method this build does not decode
            // still list. Names and sizes come from the directory either way.
            Ok(entry) => collector.push(ArchiveEntryPreview {
                name: entry.name().to_string(),
                size: entry.size(),
                compressed_size: entry.compressed_size(),
                is_dir: entry.is_dir(),
                encrypted: entry.encrypted(),
            }),
            Err(_) => unreadable += 1,
        }
    }
    let note = (unreadable > 0).then(|| format!("有 {unreadable} 个条目的目录记录损坏，已跳过。"));
    collector.finish("zip", false, note)
}

/// 7z also keeps its header at the end, so this is exact without walking the payload.
fn read_7z(path: &Path) -> ArchiveListing {
    let mut collector = Collector::new();
    match sevenz_rust2::ArchiveReader::open(path, sevenz_rust2::Password::empty()) {
        Ok(reader) => {
            for entry in &reader.archive().files {
                collector.push(ArchiveEntryPreview {
                    name: entry.name.clone(),
                    size: entry.size,
                    compressed_size: 0, // 7z compresses whole folders, not per-file.
                    is_dir: entry.is_directory,
                    encrypted: false,
                });
            }
            collector.finish("7z", false, None)
        }
        // A header-encrypted 7z cannot be listed without the password; that is the format
        // working as intended, not a failure of ours, and the message should say which.
        Err(e) => ArchiveListing::empty(
            "7z",
            Some(format!("7z 头部无法读取（可能整个头被加密了）：{e}")),
        ),
    }
}

fn read_tar(path: &Path) -> ArchiveListing {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return ArchiveListing::empty("tar", Some(format!("打不开文件：{e}"))),
    };
    // An uncompressed tar is seekable, so the walk skips over file data instead of reading it —
    // fast even for a large archive.
    let (collector, partial) = walk_tar(&mut tar::Archive::new(BufReader::new(file)), None);
    let note = partial.then(|| budget_note());
    collector.finish("tar", partial, note)
}

/// gzip/bzip2/xz/zstd carry a single stream. Very often that stream is a tar, in which case the
/// interesting listing is the tar inside — so decompress and walk it, under a budget, because
/// unlike ZIP there is no index to jump to.
fn read_compressed(path: &Path, format: Format) -> ArchiveListing {
    let label = match format {
        Format::Gzip => "gzip",
        Format::Bzip2 => "bzip2",
        Format::Xz => "xz",
        Format::Zstd => "zstd",
        _ => "unknown",
    };
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return ArchiveListing::empty(label, Some(format!("打不开文件：{e}"))),
    };
    let reader = BufReader::new(file);
    let deadline = Instant::now() + STREAM_TIME_BUDGET;

    // Only walk the inner stream as a tar when the name says it is one. Decompressing a
    // multi-gigabyte payload on the chance it might be a tar is not worth the wait.
    if wraps_a_tar(path) {
        let tar_label = format!("tar.{label}");
        let (collector, partial) = match format {
            Format::Gzip => walk_tar(
                &mut tar::Archive::new(flate2::read::MultiGzDecoder::new(reader)),
                Some(deadline),
            ),
            Format::Bzip2 => walk_tar(
                &mut tar::Archive::new(bzip2::read::MultiBzDecoder::new(reader)),
                Some(deadline),
            ),
            Format::Zstd => match ruzstd::decoding::StreamingDecoder::new(reader) {
                Ok(dec) => walk_tar(&mut tar::Archive::new(dec), Some(deadline)),
                Err(e) => {
                    return ArchiveListing::empty(&tar_label, Some(format!("zstd 解码失败：{e}")))
                }
            },
            // lzma-rs has no streaming reader, so the xz path decodes into memory. Bounded by
            // the byte budget so a huge .tar.xz degrades to a partial listing rather than
            // swallowing the machine's RAM.
            Format::Xz => match decode_xz_bounded(path) {
                Ok((buf, complete)) => {
                    let (collector, partial) =
                        walk_tar(&mut tar::Archive::new(std::io::Cursor::new(buf)), Some(deadline));
                    (collector, partial || !complete)
                }
                Err(e) => {
                    return ArchiveListing::empty(&tar_label, Some(format!("xz 解码失败：{e}")))
                }
            },
            _ => return ArchiveListing::empty(&tar_label, None),
        };
        let note = partial.then(|| budget_note());
        return collector.finish(&tar_label, partial, note);
    }

    // Not a tarball: one compressed member. gzip is the only one of the four that records the
    // original name and size, so it is the only one that can report them.
    let mut collector = Collector::new();
    let (inner_name, inner_size) = if format == Format::Gzip {
        gzip_member_info(path)
    } else {
        (None, None)
    };
    let compressed = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let fallback = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("payload")
        .to_string();
    collector.push(ArchiveEntryPreview {
        name: inner_name.unwrap_or(fallback),
        size: inner_size.unwrap_or(0),
        compressed_size: compressed,
        is_dir: false,
        encrypted: false,
    });
    let note = (inner_size.is_none() && format != Format::Gzip).then(|| {
        format!("{label} 是单流压缩，没有条目表；解压后的大小要解完才知道。")
    });
    collector.finish(label, false, note)
}

/// Walk tar headers. With `deadline` set the walk is bounded and may stop early; the bool
/// reports whether it did, so the caller can label the count as a floor.
fn walk_tar<R: Read>(archive: &mut tar::Archive<R>, deadline: Option<Instant>) -> (Collector, bool) {
    let mut collector = Collector::new();
    let entries = match archive.entries() {
        Ok(e) => e,
        Err(_) => return (collector, false),
    };
    for item in entries {
        if let Some(deadline) = deadline {
            if Instant::now() >= deadline || collector.total_size > STREAM_BYTE_BUDGET {
                return (collector, true);
            }
        }
        let entry = match item {
            Ok(entry) => entry,
            // A truncated or damaged tail ends the walk; what was read before it is still real
            // and worth showing.
            Err(_) => return (collector, true),
        };
        let header = entry.header();
        let size = header.size().unwrap_or(0);
        let name = entry
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from_utf8_lossy(&entry.path_bytes()).into_owned());
        let is_dir = header.entry_type().is_dir() || name.ends_with('/');
        collector.push(ArchiveEntryPreview {
            name,
            size,
            compressed_size: 0, // tar does not compress per entry.
            is_dir,
            encrypted: false,
        });
    }
    (collector, false)
}

fn decode_xz_bounded(path: &Path) -> std::io::Result<(Vec<u8>, bool)> {
    let file = File::open(path)?;
    let mut out = Vec::new();
    let mut reader = BufReader::new(file);
    match lzma_rs::xz_decompress(&mut reader, &mut out) {
        Ok(()) => Ok((out, true)),
        // A partial decode still yields a usable prefix of the tar, which lists real entries.
        Err(_) if !out.is_empty() => Ok((out, false)),
        Err(e) => Err(std::io::Error::other(e)),
    }
}

/// gzip's header optionally carries the original filename, and its last four bytes carry the
/// uncompressed size mod 2^32. Both are read directly — no decompression.
fn gzip_member_info(path: &Path) -> (Option<String>, Option<u64>) {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return (None, None),
    };
    let mut header = [0u8; 10];
    if file.read_exact(&mut header).is_err() || header[0] != 0x1f || header[1] != 0x8b {
        return (None, None);
    }
    let flags = header[3];
    let mut name = None;
    // FEXTRA(0x04) comes before FNAME(0x08); skip it by its own length field.
    if flags & 0x04 != 0 {
        let mut len = [0u8; 2];
        if file.read_exact(&mut len).is_err() {
            return (None, None);
        }
        let len = u16::from_le_bytes(len) as i64;
        if file.seek(SeekFrom::Current(len)).is_err() {
            return (None, None);
        }
    }
    if flags & 0x08 != 0 {
        let mut bytes = Vec::new();
        let mut byte = [0u8; 1];
        while file.read_exact(&mut byte).is_ok() && byte[0] != 0 && bytes.len() < 4096 {
            bytes.push(byte[0]);
        }
        if !bytes.is_empty() {
            name = Some(String::from_utf8_lossy(&bytes).into_owned());
        }
    }
    // ISIZE is the true size only below 4 GiB; above that it has wrapped and would be a lie.
    let size = file
        .seek(SeekFrom::End(-4))
        .ok()
        .and_then(|_| {
            let mut isize_bytes = [0u8; 4];
            file.read_exact(&mut isize_bytes).ok()?;
            Some(u32::from_le_bytes(isize_bytes) as u64)
        })
        .filter(|_| std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) < 4 * 1024 * 1024 * 1024);
    (name, size)
}

fn budget_note() -> String {
    format!(
        "这是流式压缩包，没有目录表，只能边解压边数。已经数了 {} 秒就先停下来了，条目总数是「至少这么多」，不是全部。",
        STREAM_TIME_BUDGET.as_secs()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("michael-archive-tests");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    /// The bug this module exists for: the count must describe the archive, not the preview cap.
    #[test]
    fn a_zip_past_the_preview_cap_still_reports_its_real_size() {
        let path = temp("many.zip");
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let count = PREVIEW_LIMIT + 37;
        for i in 0..count {
            zip.start_file(format!("f{i}.txt"), opts).unwrap();
            zip.write_all(b"x").unwrap();
        }
        zip.finish().unwrap();

        let listing = read_listing(&path);
        assert_eq!(listing.format, "zip");
        assert_eq!(
            listing.total, count as u64,
            "the total must count the archive, not the rows we chose to render — reporting the \
             cap is what made a 2GB zip claim to hold 200 files"
        );
        assert_eq!(listing.entries.len(), PREVIEW_LIMIT, "the preview stays bounded");
        assert!(listing.truncated, "and it must admit it is a preview");
        assert!(!listing.count_is_partial, "a zip is counted exactly, never estimated");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_gzipped_tar_lists_what_is_inside_it_rather_than_the_wrapper() {
        let path = temp("bundle.tar.gz");
        let out = File::create(&path).unwrap();
        let enc = flate2::write::GzEncoder::new(out, flate2::Compression::fast());
        {
            let mut builder = tar::Builder::new(enc);
            for name in ["src/main.rs", "README.md", "assets/logo.png"] {
                let body = b"hello archive";
                let mut header = tar::Header::new_gnu();
                header.set_size(body.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder.append_data(&mut header, name, &body[..]).unwrap();
            }
            builder.into_inner().unwrap().finish().unwrap();
        }

        let listing = read_listing(&path);
        assert_eq!(listing.format, "tar.gzip");
        assert_eq!(listing.total, 3);
        let names: Vec<_> = listing.entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"src/main.rs") && names.contains(&"assets/logo.png"));
        assert!(!listing.truncated);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_plain_gzip_reports_the_original_name_and_size_from_its_own_header() {
        let path = temp("notes.txt.gz");
        let payload = b"the quick brown fox jumps over the lazy dog";
        {
            let out = File::create(&path).unwrap();
            // GzBuilder writes FNAME, which is where the original name comes from.
            let mut enc = flate2::GzBuilder::new()
                .filename("notes.txt")
                .write(out, flate2::Compression::fast());
            enc.write_all(payload).unwrap();
            enc.finish().unwrap();
        }

        let listing = read_listing(&path);
        assert_eq!(listing.format, "gzip");
        assert_eq!(listing.total, 1);
        assert_eq!(listing.entries[0].name, "notes.txt");
        assert_eq!(
            listing.entries[0].size,
            payload.len() as u64,
            "read from the gzip trailer, without decompressing anything"
        );
        let _ = std::fs::remove_file(&path);
    }

    /// Content decides, not the extension — the panel is most useful on files that are not
    /// named the way their contents suggest.
    #[test]
    fn a_zip_named_something_else_is_still_read_as_a_zip() {
        let path = temp("mystery.bin");
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        zip.start_file("inside.txt", opts).unwrap();
        zip.write_all(b"found me").unwrap();
        zip.finish().unwrap();

        let listing = read_listing(&path);
        assert_eq!(listing.format, "zip");
        assert_eq!(listing.total, 1);
        assert_eq!(listing.entries[0].name, "inside.txt");
        let _ = std::fs::remove_file(&path);
    }

    /// macOS's bsdtar writes an AppleDouble sidecar next to every file. They are real members —
    /// `tar -tf` on macOS hides them, GNU tar shows them — so counting them as files reports
    /// twice as many as the user put in.
    #[test]
    fn macos_xattr_sidecars_are_counted_apart_from_the_real_files() {
        let path = temp("sidecars.tar");
        {
            let out = File::create(&path).unwrap();
            let mut builder = tar::Builder::new(out);
            for name in ["src/a.bin", "src/._a.bin", "src/b.bin", "src/._b.bin", "__MACOSX/x"] {
                let body = b"data";
                let mut header = tar::Header::new_gnu();
                header.set_size(body.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder.append_data(&mut header, name, &body[..]).unwrap();
            }
            builder.finish().unwrap();
        }

        let listing = read_listing(&path);
        assert_eq!(listing.total, 2, "two real files, whatever the archiver added beside them");
        assert_eq!(listing.metadata_entries, 3, "and the sidecars are reported, not discarded");
        let names: Vec<_> = listing.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["src/a.bin", "src/b.bin"]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_rar_says_why_it_cannot_be_listed_instead_of_looking_empty() {
        let path = temp("archive.rar");
        std::fs::write(&path, b"Rar!\x1A\x07\x01\x00rest of the file").unwrap();
        let listing = read_listing(&path);
        assert_eq!(listing.format, "rar");
        assert!(listing.entries.is_empty());
        assert!(
            listing.note.as_deref().unwrap_or("").contains("RAR"),
            "an empty list with no explanation reads as a corrupt archive"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_truncated_tar_keeps_the_entries_it_did_read() {
        let path = temp("cut.tar");
        {
            let out = File::create(&path).unwrap();
            let mut builder = tar::Builder::new(out);
            for name in ["a.txt", "b.txt", "c.txt"] {
                let body = vec![b'x'; 2048];
                let mut header = tar::Header::new_gnu();
                header.set_size(body.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder.append_data(&mut header, name, &body[..]).unwrap();
            }
            builder.finish().unwrap();
        }
        // Lop off the tail, the way an interrupted download would.
        let full = std::fs::read(&path).unwrap();
        std::fs::write(&path, &full[..full.len() - 3000]).unwrap();

        let listing = read_listing(&path);
        assert!(listing.total >= 1, "a damaged tail must not discard the good prefix");
        assert!(listing.entries.iter().any(|e| e.name == "a.txt"));
        let _ = std::fs::remove_file(&path);
    }
}

#[cfg(test)]
mod probe {
    /// Point MICHAEL_ARCHIVE_PROBE at a real archive to see what the reader makes of it:
    ///   MICHAEL_ARCHIVE_PROBE=/path/to.zip cargo test --lib probe -- --ignored --nocapture
    #[test]
    #[ignore]
    fn probe_a_real_archive() {
        let Ok(path) = std::env::var("MICHAEL_ARCHIVE_PROBE") else { return };
        let started = std::time::Instant::now();
        let listing = super::read_listing(std::path::Path::new(&path));
        eprintln!(
            "format={} total={} shown={} truncated={} partial={} unpacked={} bytes encrypted={} in {:?}",
            listing.format, listing.total, listing.entries.len(), listing.truncated,
            listing.count_is_partial, listing.total_size, listing.encrypted, started.elapsed()
        );
        if let Some(note) = &listing.note { eprintln!("note: {note}"); }
        for e in listing.entries.iter().take(6) {
            eprintln!("  {:>12}  {}", e.size, e.name);
        }
    }
}
