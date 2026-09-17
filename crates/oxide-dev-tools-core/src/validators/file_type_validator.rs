//! File type detection by content sniffing (Tika-style).
//!
//! [`detect_file_type`] identifies a byte stream's format without trusting
//! its file extension, using the same layered strategy as Apache Tika:
//!
//! 1. Container probing — ZIP archives are searched for central-directory
//!    entry names (OOXML, ODF, EPUB, JAR), and RIFF / EBML / ISO BMFF
//!    containers are identified by their inner four-byte type.
//! 2. Magic-byte matching — a static signature table with offsets and masks
//!    covers documents, images, audio, video, archives, executables, and
//!    binary data formats; the longest matching signature wins.
//! 3. Text heuristics — BOM detection, UTF-8 validity, control-byte
//!    density, and printable-byte ratios decide whether the stream is text,
//!    then JSON / XML / HTML / SVG / PEM content probes refine the kind.
//! 4. Extension fallback — when content is unrecognized, a known file
//!    extension produces a low-confidence result.
//!
//! Detection never fails: unrecognized data is reported as
//! [`FileTypeKind::Unknown`]. No I/O is performed here — callers pass the
//! bytes they already read.

use quick_xml::Reader;
use quick_xml::events::Event;
use serde_json::Value as JsonValue;

// -------- Public API --------

/// A detected file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTypeKind {
    /// PDF document.
    Pdf,
    /// Rich Text Format document.
    Rtf,
    /// OOXML Word document (ZIP container).
    Docx,
    /// OOXML spreadsheet (ZIP container).
    Xlsx,
    /// OOXML presentation (ZIP container).
    Pptx,
    /// OpenDocument text (ZIP container).
    Odt,
    /// OpenDocument spreadsheet (ZIP container).
    Ods,
    /// OpenDocument presentation (ZIP container).
    Odp,
    /// EPUB electronic book (ZIP container).
    Epub,
    /// Java archive (ZIP container).
    Jar,
    /// PNG image.
    Png,
    /// JPEG image.
    Jpeg,
    /// GIF image.
    Gif,
    /// Windows bitmap.
    Bmp,
    /// WebP image (RIFF container).
    WebP,
    /// TIFF image.
    Tiff,
    /// ICO icon.
    Ico,
    /// SVG vector image (text).
    Svg,
    /// Adobe Photoshop document.
    Psd,
    /// MP3 audio.
    Mp3,
    /// WAVE audio (RIFF container).
    Wav,
    /// FLAC audio.
    Flac,
    /// Ogg audio.
    Ogg,
    /// AAC (ADTS) audio.
    Aac,
    /// MPEG-4 audio (ISO BMFF container).
    M4a,
    /// MIDI audio.
    Midi,
    /// MPEG-4 video (ISO BMFF container).
    Mp4,
    /// WebM or Matroska video (EBML container).
    WebM,
    /// AVI video (RIFF container).
    Avi,
    /// ZIP archive without a recognized document layout.
    Zip,
    /// Gzip-compressed data.
    Gzip,
    /// 7-Zip archive.
    SevenZip,
    /// RAR archive.
    Rar,
    /// Tar archive.
    Tar,
    /// Bzip2-compressed data.
    Bzip2,
    /// XZ-compressed data.
    Xz,
    /// ELF executable or object file.
    Elf,
    /// PE (Windows) executable or DLL.
    Pe,
    /// Mach-O executable or library.
    MachO,
    /// WebAssembly module.
    Wasm,
    /// SQLite database.
    Sqlite,
    /// TrueType font.
    Ttf,
    /// OpenType (CFF) font.
    Otf,
    /// WOFF 1.0 web font.
    Woff,
    /// WOFF 2.0 web font.
    Woff2,
    /// JSON document (text).
    Json,
    /// XML document (text).
    Xml,
    /// HTML document (text).
    Html,
    /// Plain text (including source code and scripts).
    PlainText,
    /// PEM-encoded certificate or key (text).
    Pem,
    /// Nothing was recognized.
    Unknown,
}

impl FileTypeKind {
    /// Lowercase CLI name of the type.
    pub fn name(self) -> &'static str {
        self.metadata().0
    }

    /// MIME type of the result; `None` for unknown data.
    pub fn mime(self) -> Option<&'static str> {
        self.metadata().1
    }

    /// File extensions conventionally used for the type.
    pub fn extensions(self) -> &'static [&'static str] {
        self.metadata().2
    }

    fn metadata(self) -> (&'static str, Option<&'static str>, &'static [&'static str]) {
        match self {
            FileTypeKind::Pdf => ("pdf", Some("application/pdf"), &["pdf"]),
            FileTypeKind::Rtf => ("rtf", Some("application/rtf"), &["rtf"]),
            FileTypeKind::Docx => {
                ("docx", Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"), &["docx"])
            }
            FileTypeKind::Xlsx => {
                ("xlsx", Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"), &["xlsx"])
            }
            FileTypeKind::Pptx => {
                ("pptx", Some("application/vnd.openxmlformats-officedocument.presentationml.presentation"), &["pptx"])
            }
            FileTypeKind::Odt => ("odt", Some("application/vnd.oasis.opendocument.text"), &["odt"]),
            FileTypeKind::Ods => ("ods", Some("application/vnd.oasis.opendocument.spreadsheet"), &["ods"]),
            FileTypeKind::Odp => ("odp", Some("application/vnd.oasis.opendocument.presentation"), &["odp"]),
            FileTypeKind::Epub => ("epub", Some("application/epub+zip"), &["epub"]),
            FileTypeKind::Jar => ("jar", Some("application/java-archive"), &["jar"]),
            FileTypeKind::Png => ("png", Some("image/png"), &["png"]),
            FileTypeKind::Jpeg => ("jpeg", Some("image/jpeg"), &["jpg", "jpeg"]),
            FileTypeKind::Gif => ("gif", Some("image/gif"), &["gif"]),
            FileTypeKind::Bmp => ("bmp", Some("image/bmp"), &["bmp"]),
            FileTypeKind::WebP => ("webp", Some("image/webp"), &["webp"]),
            FileTypeKind::Tiff => ("tiff", Some("image/tiff"), &["tif", "tiff"]),
            FileTypeKind::Ico => ("ico", Some("image/x-icon"), &["ico"]),
            FileTypeKind::Svg => ("svg", Some("image/svg+xml"), &["svg"]),
            FileTypeKind::Psd => ("psd", Some("image/vnd.adobe.photoshop"), &["psd"]),
            FileTypeKind::Mp3 => ("mp3", Some("audio/mpeg"), &["mp3"]),
            FileTypeKind::Wav => ("wav", Some("audio/wav"), &["wav"]),
            FileTypeKind::Flac => ("flac", Some("audio/flac"), &["flac"]),
            FileTypeKind::Ogg => ("ogg", Some("audio/ogg"), &["ogg", "oga"]),
            FileTypeKind::Aac => ("aac", Some("audio/aac"), &["aac"]),
            FileTypeKind::M4a => ("m4a", Some("audio/mp4"), &["m4a"]),
            FileTypeKind::Midi => ("midi", Some("audio/midi"), &["mid", "midi"]),
            FileTypeKind::Mp4 => ("mp4", Some("video/mp4"), &["mp4", "m4v"]),
            FileTypeKind::WebM => ("webm", Some("video/webm"), &["webm", "mkv"]),
            FileTypeKind::Avi => ("avi", Some("video/x-msvideo"), &["avi"]),
            FileTypeKind::Zip => ("zip", Some("application/zip"), &["zip"]),
            FileTypeKind::Gzip => ("gzip", Some("application/gzip"), &["gz", "gzip"]),
            FileTypeKind::SevenZip => ("7z", Some("application/x-7z-compressed"), &["7z"]),
            FileTypeKind::Rar => ("rar", Some("application/vnd.rar"), &["rar"]),
            FileTypeKind::Tar => ("tar", Some("application/x-tar"), &["tar"]),
            FileTypeKind::Bzip2 => ("bzip2", Some("application/x-bzip2"), &["bz2"]),
            FileTypeKind::Xz => ("xz", Some("application/x-xz"), &["xz"]),
            FileTypeKind::Elf => ("elf", Some("application/x-elf"), &["elf", "so", "o"]),
            FileTypeKind::Pe => ("pe", Some("application/vnd.microsoft.portable-executable"), &["exe", "dll"]),
            FileTypeKind::MachO => ("mach-o", Some("application/x-mach-binary"), &["dylib", "macho"]),
            FileTypeKind::Wasm => ("wasm", Some("application/wasm"), &["wasm"]),
            FileTypeKind::Sqlite => ("sqlite", Some("application/vnd.sqlite3"), &["sqlite", "sqlite3", "db"]),
            FileTypeKind::Ttf => ("ttf", Some("font/ttf"), &["ttf"]),
            FileTypeKind::Otf => ("otf", Some("font/otf"), &["otf"]),
            FileTypeKind::Woff => ("woff", Some("font/woff"), &["woff"]),
            FileTypeKind::Woff2 => ("woff2", Some("font/woff2"), &["woff2"]),
            FileTypeKind::Json => ("json", Some("application/json"), &["json"]),
            FileTypeKind::Xml => ("xml", Some("application/xml"), &["xml"]),
            FileTypeKind::Html => ("html", Some("text/html"), &["html", "htm"]),
            FileTypeKind::PlainText => (
                "plain-text",
                Some("text/plain"),
                &["txt", "md", "rs", "log", "csv", "yml", "yaml", "toml", "ini", "sh"],
            ),
            FileTypeKind::Pem => ("pem", Some("application/x-pem-file"), &["pem", "crt", "key", "cer"]),
            FileTypeKind::Unknown => ("unknown", None, &[]),
        }
    }
}

/// Every detectable kind (excluding [`FileTypeKind::Unknown`]), used for the
/// extension fallback and metadata coverage tests.
const ALL_KINDS: &[FileTypeKind] = &[
    FileTypeKind::Pdf,
    FileTypeKind::Rtf,
    FileTypeKind::Docx,
    FileTypeKind::Xlsx,
    FileTypeKind::Pptx,
    FileTypeKind::Odt,
    FileTypeKind::Ods,
    FileTypeKind::Odp,
    FileTypeKind::Epub,
    FileTypeKind::Jar,
    FileTypeKind::Png,
    FileTypeKind::Jpeg,
    FileTypeKind::Gif,
    FileTypeKind::Bmp,
    FileTypeKind::WebP,
    FileTypeKind::Tiff,
    FileTypeKind::Ico,
    FileTypeKind::Svg,
    FileTypeKind::Psd,
    FileTypeKind::Mp3,
    FileTypeKind::Wav,
    FileTypeKind::Flac,
    FileTypeKind::Ogg,
    FileTypeKind::Aac,
    FileTypeKind::M4a,
    FileTypeKind::Midi,
    FileTypeKind::Mp4,
    FileTypeKind::WebM,
    FileTypeKind::Avi,
    FileTypeKind::Zip,
    FileTypeKind::Gzip,
    FileTypeKind::SevenZip,
    FileTypeKind::Rar,
    FileTypeKind::Tar,
    FileTypeKind::Bzip2,
    FileTypeKind::Xz,
    FileTypeKind::Elf,
    FileTypeKind::Pe,
    FileTypeKind::MachO,
    FileTypeKind::Wasm,
    FileTypeKind::Sqlite,
    FileTypeKind::Ttf,
    FileTypeKind::Otf,
    FileTypeKind::Woff,
    FileTypeKind::Woff2,
    FileTypeKind::Json,
    FileTypeKind::Xml,
    FileTypeKind::Html,
    FileTypeKind::PlainText,
    FileTypeKind::Pem,
];

/// How strongly the detection is supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileConfidence {
    /// Magic bytes or a container structure matched.
    High,
    /// Text heuristics identified the content.
    Medium,
    /// Only the file extension was recognized.
    Low,
    /// Nothing was recognized.
    None,
}

/// Options for [`detect_file_type`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileTypeOptions {
    /// The bytes to classify (already read by the caller).
    pub data: Vec<u8>,
    /// File name used for the extension cross-check; `None` skips it.
    pub file_name: Option<String>,
    /// Expected type; when set, the report compares it with the result.
    pub expected: Option<FileTypeKind>,
}

/// The result of detecting a file's type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTypeReport {
    /// The detected type; [`FileTypeKind::Unknown`] when nothing matched.
    pub detected: FileTypeKind,
    /// The MIME type of the result; `None` for unknown data.
    pub mime_type: Option<&'static str>,
    /// How strongly the result is supported.
    pub confidence: FileConfidence,
    /// The extension taken from the file name (lowercased, without the dot).
    pub extension: Option<String>,
    /// Whether `detected` agrees with the extension; `None` when there is no
    /// extension or the type is unknown.
    pub extension_matches: Option<bool>,
    /// Whether `detected` equals the expected type; `None` when no
    /// expectation was configured.
    pub expected_matches: Option<bool>,
    /// Detector layers that contributed to the result.
    pub layers: Vec<&'static str>,
}

/// Detect the type of a byte stream and produce a detailed report.
///
/// Detection never fails and never panics: unrecognized data is reported as
/// [`FileTypeKind::Unknown`] with [`FileConfidence::None`].
pub fn detect_file_type(options: &FileTypeOptions) -> FileTypeReport {
    let mut report = FileTypeReport {
        detected: FileTypeKind::Unknown,
        mime_type: None,
        confidence: FileConfidence::None,
        extension: extension_of(options.file_name.as_deref()),
        extension_matches: None,
        expected_matches: None,
        layers: Vec::new(),
    };
    if options.data.is_empty() {
        report.layers.push("empty");
    } else if let Some(kind) = detect_container(&options.data, &mut report) {
        set_detection(&mut report, kind, FileConfidence::High);
    } else if let Some(kind) = find_magic(&options.data, SIGNATURES, &mut report) {
        set_detection(&mut report, kind, FileConfidence::High);
    } else if let Some(kind) = detect_text(&options.data, &mut report) {
        set_detection(&mut report, kind, FileConfidence::Medium);
    } else {
        let extension = report.extension.clone();
        if let Some(kind) = detect_by_extension(extension.as_deref(), &mut report) {
            set_detection(&mut report, kind, FileConfidence::Low);
        }
    }
    finalize_report(&mut report, options);
    report
}

// -------- Report helpers --------

fn set_detection(report: &mut FileTypeReport, kind: FileTypeKind, confidence: FileConfidence) {
    report.detected = kind;
    report.mime_type = kind.mime();
    report.confidence = confidence;
}

fn finalize_report(report: &mut FileTypeReport, options: &FileTypeOptions) {
    report.extension_matches = match (&report.extension, report.detected) {
        (Some(extension), kind) if kind != FileTypeKind::Unknown => {
            Some(kind.extensions().contains(&extension.as_str()))
        }
        _ => None,
    };
    report.expected_matches = options.expected.map(|expected| expected == report.detected);
}

/// Extract the lowercased extension (without the dot) from a file name.
fn extension_of(file_name: Option<&str>) -> Option<String> {
    let (_, extension) = file_name?.rsplit_once('.')?;
    if extension.is_empty() || extension.len() > 16 {
        return None;
    }
    Some(extension.to_ascii_lowercase())
}

// -------- Container probing --------

fn detect_container(data: &[u8], report: &mut FileTypeReport) -> Option<FileTypeKind> {
    if starts_with(data, b"PK\x03\x04") || starts_with(data, b"PK\x05\x06") || starts_with(data, b"PK\x07\x08") {
        report.layers.push("container");
        report.layers.push("zip");
        return Some(probe_zip(data, report));
    }
    if starts_with(data, &[0x1F, 0x8B, 0x08]) {
        report.layers.push("container");
        report.layers.push("gzip");
        return Some(FileTypeKind::Gzip);
    }
    if starts_with(data, b"RIFF") && data.len() >= 12 {
        report.layers.push("container");
        report.layers.push("riff");
        return match &data[8..12] {
            b"WEBP" => Some(FileTypeKind::WebP),
            b"WAVE" => Some(FileTypeKind::Wav),
            b"AVI " => Some(FileTypeKind::Avi),
            _ => None,
        };
    }
    if starts_with(data, &[0x1A, 0x45, 0xDF, 0xA3]) {
        report.layers.push("container");
        report.layers.push("ebml");
        return Some(FileTypeKind::WebM);
    }
    if data.len() >= 12 && &data[4..8] == b"ftyp" {
        report.layers.push("container");
        report.layers.push("bmff");
        return match &data[8..12] {
            b"M4A " | b"M4B " | b"F4A " => Some(FileTypeKind::M4a),
            _ => Some(FileTypeKind::Mp4),
        };
    }
    None
}

/// Classify a ZIP archive by the entry names found anywhere in the data
/// (entry names live in the central directory, which is usually at the end).
fn probe_zip(data: &[u8], report: &mut FileTypeReport) -> FileTypeKind {
    if contains_marker(data, b"mimetypeapplication/vnd.oasis.opendocument") {
        report.layers.push("odf");
        if contains_marker(data, b".presentation") {
            return FileTypeKind::Odp;
        }
        if contains_marker(data, b".spreadsheet") {
            return FileTypeKind::Ods;
        }
        return FileTypeKind::Odt;
    }
    if contains_marker(data, b"mimetypeapplication/epub+zip") {
        report.layers.push("epub");
        return FileTypeKind::Epub;
    }
    if contains_marker(data, b"[Content_Types].xml") {
        report.layers.push("ooxml");
        if contains_marker(data, b"word/") {
            return FileTypeKind::Docx;
        }
        if contains_marker(data, b"xl/") {
            return FileTypeKind::Xlsx;
        }
        if contains_marker(data, b"ppt/") {
            return FileTypeKind::Pptx;
        }
        return FileTypeKind::Docx;
    }
    if contains_marker(data, b"META-INF/MANIFEST.MF") {
        report.layers.push("jar");
        return FileTypeKind::Jar;
    }
    FileTypeKind::Zip
}

/// Whether the exact byte marker occurs anywhere in the data.
fn contains_marker(data: &[u8], marker: &[u8]) -> bool {
    data.windows(marker.len()).any(|window| window == marker)
}

fn starts_with(data: &[u8], prefix: &[u8]) -> bool {
    data.len() >= prefix.len() && &data[..prefix.len()] == prefix
}

// -------- Magic-byte matching --------

/// One entry of the magic-byte signature table.
struct Signature {
    kind: FileTypeKind,
    /// Byte offset of the magic value within the data.
    offset: usize,
    /// Expected bytes; wildcard bits are controlled by `mask`.
    magic: &'static [u8],
    /// Per-byte mask applied before comparing; `None` requires an exact
    /// match. Masked entries only match when `magic & mask == magic`.
    mask: Option<&'static [u8]>,
}

const fn sig(kind: FileTypeKind, offset: usize, magic: &'static [u8], mask: Option<&'static [u8]>) -> Signature {
    Signature {
        kind,
        offset,
        magic,
        mask,
    }
}

/// Signature table ordered by format group; the matcher prefers the entry
/// with the longest magic value regardless of table order.
const SIGNATURES: &[Signature] = &[
    // Documents
    sig(FileTypeKind::Pdf, 0, b"%PDF-", None),
    sig(FileTypeKind::Rtf, 0, b"{\\rtf", None),
    // Images
    sig(FileTypeKind::Png, 0, b"\x89PNG\r\n\x1A\n", None),
    sig(FileTypeKind::Jpeg, 0, b"\xFF\xD8\xFF", None),
    sig(FileTypeKind::Gif, 0, b"GIF8", None),
    sig(FileTypeKind::Bmp, 0, b"BM", None),
    sig(FileTypeKind::Tiff, 0, b"II*\x00", None),
    sig(FileTypeKind::Tiff, 0, b"MM\x00*", None),
    sig(FileTypeKind::Ico, 0, b"\x00\x00\x01\x00", None),
    sig(FileTypeKind::Ico, 0, b"\x00\x00\x02\x00", None),
    sig(FileTypeKind::Psd, 0, b"8BPS", None),
    // Audio
    sig(FileTypeKind::Mp3, 0, b"ID3", None),
    // MPEG-1 Layer III frame sync (version bit wildcarded).
    sig(FileTypeKind::Mp3, 0, &[0xFF, 0xFA], Some(&[0xFF, 0xFE])),
    sig(FileTypeKind::Flac, 0, b"fLaC", None),
    sig(FileTypeKind::Ogg, 0, b"OggS", None),
    // ADTS sync word: 12 one-bits, then the MPEG version ID bit and the
    // layer bits (always 00 for ADTS); the protection bit is wildcarded.
    sig(FileTypeKind::Aac, 0, &[0xFF, 0xF0], Some(&[0xFF, 0xF6])),
    sig(FileTypeKind::Midi, 0, b"MThd", None),
    // Archives
    sig(FileTypeKind::SevenZip, 0, b"7z\xBC\xAF\x27\x1C", None),
    sig(FileTypeKind::Rar, 0, b"Rar!\x1A\x07\x00", None),
    sig(FileTypeKind::Rar, 0, b"Rar!\x1A\x07\x01\x00", None),
    sig(FileTypeKind::Tar, 257, b"ustar", None),
    sig(FileTypeKind::Bzip2, 0, b"BZh", None),
    sig(FileTypeKind::Xz, 0, b"\xFD7zXZ\x00", None),
    // Executables
    sig(FileTypeKind::Elf, 0, b"\x7FELF", None),
    sig(FileTypeKind::Pe, 0, b"MZ", None),
    sig(FileTypeKind::MachO, 0, b"\xFE\xED\xFA\xCE", None),
    sig(FileTypeKind::MachO, 0, b"\xFE\xED\xFA\xCF", None),
    sig(FileTypeKind::MachO, 0, b"\xCE\xFA\xED\xFE", None),
    sig(FileTypeKind::MachO, 0, b"\xCF\xFA\xED\xFE", None),
    sig(FileTypeKind::MachO, 0, b"\xCA\xFE\xBA\xBE", None),
    sig(FileTypeKind::Wasm, 0, b"\x00asm", None),
    // Data
    sig(FileTypeKind::Sqlite, 0, b"SQLite format 3\x00", None),
    sig(FileTypeKind::Ttf, 0, b"\x00\x01\x00\x00", None),
    sig(FileTypeKind::Otf, 0, b"OTTO", None),
    sig(FileTypeKind::Ttf, 0, b"true", None),
    sig(FileTypeKind::Ttf, 0, b"ttcf", None),
    sig(FileTypeKind::Woff, 0, b"wOFF", None),
    sig(FileTypeKind::Woff2, 0, b"wOF2", None),
];

fn find_magic(data: &[u8], table: &[Signature], report: &mut FileTypeReport) -> Option<FileTypeKind> {
    let mut best: Option<&Signature> = None;
    for candidate in table {
        let longer = best.is_none_or(|current| candidate.magic.len() > current.magic.len());
        if longer && matches_signature(data, candidate) {
            best = Some(candidate);
        }
    }
    let matched = best?;
    report.layers.push("magic");
    Some(matched.kind)
}

fn matches_signature(data: &[u8], signature: &Signature) -> bool {
    if data.len() < signature.offset + signature.magic.len() {
        return false;
    }
    let window = &data[signature.offset..signature.offset + signature.magic.len()];
    match signature.mask {
        Some(mask) => window
            .iter()
            .zip(signature.magic)
            .zip(mask)
            .all(|((byte, magic), mask_byte)| byte & mask_byte == *magic),
        None => window == signature.magic,
    }
}

// -------- Text heuristics --------

fn detect_text(data: &[u8], report: &mut FileTypeReport) -> Option<FileTypeKind> {
    let sample = &data[..data.len().min(4096)];
    if !has_utf_bom(data) && !looks_like_text(sample) {
        return None;
    }
    report.layers.push("text");
    probe_text_kind(data, report)
}

/// Whether the sample reads as human text: valid UTF-8 without control
/// characters, or (for non-UTF-8 encodings) a high printable-byte ratio.
fn looks_like_text(sample: &[u8]) -> bool {
    if sample.is_empty() {
        return false;
    }
    match std::str::from_utf8(sample) {
        Ok(text) => !text
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\t' | '\n' | '\r')),
        Err(_) => printable_ratio(sample) >= 0.95,
    }
}

fn has_utf_bom(data: &[u8]) -> bool {
    data.starts_with(&[0xEF, 0xBB, 0xBF])
        || data.starts_with(&[0xFF, 0xFE])
        || data.starts_with(&[0xFE, 0xFF])
        || data.starts_with(&[0xFF, 0xFE, 0x00, 0x00])
        || data.starts_with(&[0x00, 0x00, 0xFE, 0xFF])
}

fn printable_ratio(sample: &[u8]) -> f64 {
    if sample.is_empty() {
        return 0.0;
    }
    let printable = sample
        .iter()
        .filter(|byte| byte.is_ascii_graphic() || matches!(**byte, b' ' | b'\t' | b'\n' | b'\r'))
        .count();
    printable as f64 / sample.len() as f64
}

/// Refine a text stream into a concrete kind by probing its leading content.
fn probe_text_kind(data: &[u8], report: &mut FileTypeReport) -> Option<FileTypeKind> {
    let head = &data[..data.len().min(4096)];
    let text = String::from_utf8_lossy(head);
    let trimmed = text.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if serde_json::from_slice::<JsonValue>(data).is_ok() {
            report.layers.push("json");
            return Some(FileTypeKind::Json);
        }
        return Some(FileTypeKind::PlainText);
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("<?xml") && lower.contains("<svg") {
        report.layers.push("svg");
        return Some(FileTypeKind::Svg);
    }
    if lower.starts_with("<?xml") {
        report.layers.push("xml");
        return Some(FileTypeKind::Xml);
    }
    if lower.starts_with("<svg") {
        report.layers.push("svg");
        return Some(FileTypeKind::Svg);
    }
    if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
        report.layers.push("html");
        return Some(FileTypeKind::Html);
    }
    if lower.starts_with("-----begin") {
        report.layers.push("pem");
        return Some(FileTypeKind::Pem);
    }
    if trimmed.starts_with('<') && is_well_formed_xml(data) {
        report.layers.push("xml");
        return Some(FileTypeKind::Xml);
    }
    Some(FileTypeKind::PlainText)
}

/// Cheap XML well-formedness scan used to confirm tag-shaped text.
fn is_well_formed_xml(data: &[u8]) -> bool {
    let mut reader = Reader::from_reader(data);
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Eof) => return true,
            Ok(_) => {}
            Err(_) => return false,
        }
    }
}

// -------- Extension fallback --------

fn detect_by_extension(extension: Option<&str>, report: &mut FileTypeReport) -> Option<FileTypeKind> {
    let kind = kind_for_extension(extension?)?;
    report.layers.push("extension");
    Some(kind)
}

fn kind_for_extension(extension: &str) -> Option<FileTypeKind> {
    ALL_KINDS
        .iter()
        .copied()
        .find(|kind| kind.extensions().contains(&extension))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::sample_file_generator::{
        JpgOptions, PdfOptions, PngOptions, SampleKind, TamperKind, generate_sample_file,
    };

    fn options(data: &[u8]) -> FileTypeOptions {
        FileTypeOptions {
            data: data.to_vec(),
            ..FileTypeOptions::default()
        }
    }

    fn detect(data: &[u8]) -> FileTypeReport {
        detect_file_type(&options(data))
    }

    fn fixture(offset: usize, magic: &[u8]) -> Vec<u8> {
        let mut data = vec![0u8; offset];
        data.extend_from_slice(magic);
        data
    }

    // -------- Entry point --------

    #[test]
    fn empty_data_is_unknown() {
        let report = detect(&[]);
        assert_eq!(report.detected, FileTypeKind::Unknown);
        assert_eq!(report.confidence, FileConfidence::None);
        assert_eq!(report.mime_type, None);
        assert!(report.layers.contains(&"empty"));
    }

    #[test]
    fn layers_record_the_detection_path() {
        assert!(detect(b"\x89PNG\r\n\x1A\n").layers.contains(&"magic"));
        assert!(detect(b"hello").layers.contains(&"text"));
        assert!(detect(&[]).layers.contains(&"empty"));
    }

    // -------- Magic table --------

    #[test]
    fn every_signature_matches_its_magic() {
        for signature in SIGNATURES {
            let data = fixture(signature.offset, signature.magic);
            let report = detect(&data);
            assert_eq!(
                report.detected,
                signature.kind,
                "signature for {} at offset {} failed",
                signature.kind.name(),
                signature.offset
            );
            assert_eq!(report.confidence, FileConfidence::High);
        }
    }

    #[test]
    fn find_magic_prefers_the_longest_match() {
        let table = [
            Signature {
                kind: FileTypeKind::Bmp,
                offset: 0,
                magic: b"BM",
                mask: None,
            },
            Signature {
                kind: FileTypeKind::Bzip2,
                offset: 0,
                magic: b"BZh",
                mask: None,
            },
        ];
        let mut report = FileTypeReport {
            detected: FileTypeKind::Unknown,
            mime_type: None,
            confidence: FileConfidence::None,
            extension: None,
            extension_matches: None,
            expected_matches: None,
            layers: Vec::new(),
        };
        assert_eq!(find_magic(b"BZh91AY", &table, &mut report), Some(FileTypeKind::Bzip2));
        assert!(report.layers.contains(&"magic"));
    }

    #[test]
    fn masked_signatures_reject_non_matching_variants() {
        // MPEG-1 Layer III frame sync: second byte must have its layer bits
        // set (0xFA/0xFB), not just any 0xFF-prefixed pair.
        assert_eq!(detect(&[0xFF, 0xFB]).detected, FileTypeKind::Mp3);
        assert_eq!(detect(&[0xFF, 0xE0]).detected, FileTypeKind::Unknown);
    }

    // -------- Containers --------

    #[test]
    fn riff_containers_are_distinguished() {
        assert_eq!(detect(b"RIFF\x24\x00\x00\x00WEBPVP8 ").detected, FileTypeKind::WebP);
        assert_eq!(detect(b"RIFF\x24\x00\x00\x00WAVEfmt ").detected, FileTypeKind::Wav);
        assert_eq!(detect(b"RIFF\x24\x00\x00\x00AVI LIST").detected, FileTypeKind::Avi);
    }

    #[test]
    fn zip_archives_are_probed_for_document_layouts() {
        assert_eq!(detect(b"PK\x03\x04filler").detected, FileTypeKind::Zip);
        assert_eq!(detect(b"PK\x05\x06filler").detected, FileTypeKind::Zip);
        assert_eq!(detect(b"PK\x03\x04[Content_Types].xml word/document.xml").detected, FileTypeKind::Docx);
        assert_eq!(detect(b"PK\x03\x04[Content_Types].xml xl/workbook.xml").detected, FileTypeKind::Xlsx);
        assert_eq!(detect(b"PK\x03\x04[Content_Types].xml ppt/slides.xml").detected, FileTypeKind::Pptx);
        assert_eq!(detect(b"PK\x03\x04mimetypeapplication/vnd.oasis.opendocument.text").detected, FileTypeKind::Odt);
        assert_eq!(
            detect(b"PK\x03\x04mimetypeapplication/vnd.oasis.opendocument.spreadsheet").detected,
            FileTypeKind::Ods
        );
        assert_eq!(
            detect(b"PK\x03\x04mimetypeapplication/vnd.oasis.opendocument.presentation").detected,
            FileTypeKind::Odp
        );
        assert_eq!(detect(b"PK\x03\x04mimetypeapplication/epub+zip").detected, FileTypeKind::Epub);
        assert_eq!(detect(b"PK\x03\x04META-INF/MANIFEST.MF").detected, FileTypeKind::Jar);
    }

    #[test]
    fn multimedia_containers_are_distinguished() {
        assert_eq!(detect(b"\x00\x00\x00\x18ftypM4A \x00\x00\x02\x00").detected, FileTypeKind::M4a);
        assert_eq!(detect(b"\x00\x00\x00\x18ftypisom\x00\x00\x02\x00").detected, FileTypeKind::Mp4);
        assert_eq!(detect(b"\x1A\x45\xDF\xA3\x00\x00\x00\x00webm").detected, FileTypeKind::WebM);
        assert_eq!(detect(b"\x1F\x8B\x08\x00\x00\x00\x00\x00\x00\x03").detected, FileTypeKind::Gzip);
    }

    // -------- Text heuristics --------

    #[test]
    fn text_content_is_sniffed() {
        assert_eq!(detect(br#"{"a": 1}"#).detected, FileTypeKind::Json);
        assert_eq!(detect(b"[1, 2, 3]").detected, FileTypeKind::Json);
        assert_eq!(detect(b"{broken").detected, FileTypeKind::PlainText);
        assert_eq!(detect(br#"<?xml version="1.0"?><root/>"#).detected, FileTypeKind::Xml);
        assert_eq!(detect(b"<root><child/></root>").detected, FileTypeKind::Xml);
        assert_eq!(detect(br#"<?xml version="1.0"?><svg xmlns="x"/>"#).detected, FileTypeKind::Svg);
        assert_eq!(detect(br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#).detected, FileTypeKind::Svg);
        assert_eq!(detect(b"<!DOCTYPE html><html></html>").detected, FileTypeKind::Html);
        assert_eq!(detect(b"<HTML><BODY/></HTML>").detected, FileTypeKind::Html);
        assert_eq!(detect(b"-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----").detected, FileTypeKind::Pem);
        assert_eq!(detect(b"The quick brown fox jumps over the lazy dog.").detected, FileTypeKind::PlainText);
        assert_eq!(detect(b"#!/bin/sh\necho hello").detected, FileTypeKind::PlainText);
        assert_eq!(detect(b"\xEF\xBB\xBFhello").detected, FileTypeKind::PlainText);
        assert_eq!(detect(b"\xFF\xFEh\x00i\x00").detected, FileTypeKind::PlainText);
    }

    #[test]
    fn text_sniffing_reports_medium_confidence() {
        let report = detect(b"plain text content");
        assert_eq!(report.confidence, FileConfidence::Medium);
    }

    // -------- Unknown data --------

    #[test]
    fn unrecognized_binary_is_unknown() {
        assert_eq!(detect(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05]).detected, FileTypeKind::Unknown);
        assert_eq!(detect(&[0xAB, 0xCD, 0xEF, 0x00, 0x11]).detected, FileTypeKind::Unknown);
    }

    // -------- Extension fallback and cross-check --------

    #[test]
    fn extension_fallback_covers_unknown_content() {
        let report = detect_file_type(&FileTypeOptions {
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
            file_name: Some("photo.png".to_string()),
            ..FileTypeOptions::default()
        });
        assert_eq!(report.detected, FileTypeKind::Png);
        assert_eq!(report.confidence, FileConfidence::Low);
        assert!(report.layers.contains(&"extension"));
    }

    #[test]
    fn extension_cross_check_compares_content_and_name() {
        let png = b"\x89PNG\r\n\x1A\n";

        let report = detect_file_type(&FileTypeOptions {
            data: png.to_vec(),
            file_name: Some("photo.png".to_string()),
            ..FileTypeOptions::default()
        });
        assert_eq!(report.extension_matches, Some(true));

        let report = detect_file_type(&FileTypeOptions {
            data: png.to_vec(),
            file_name: Some("photo.jpg".to_string()),
            ..FileTypeOptions::default()
        });
        assert_eq!(report.extension_matches, Some(false));

        let report = detect_file_type(&FileTypeOptions {
            data: png.to_vec(),
            ..FileTypeOptions::default()
        });
        assert_eq!(report.extension, None);
        assert_eq!(report.extension_matches, None);

        let report = detect_file_type(&FileTypeOptions {
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
            file_name: Some("unknown.xyz".to_string()),
            ..FileTypeOptions::default()
        });
        assert_eq!(report.detected, FileTypeKind::Unknown);
        assert_eq!(report.extension_matches, None);
    }

    #[test]
    fn extension_is_extracted_from_file_names() {
        assert_eq!(extension_of(Some("photo.png")), Some("png".to_string()));
        assert_eq!(extension_of(Some("archive.tar.gz")), Some("gz".to_string()));
        assert_eq!(extension_of(Some("dir/deep/PHOTO.JPG")), Some("jpg".to_string()));
        assert_eq!(extension_of(Some("README")), None);
        assert_eq!(extension_of(Some("photo.")), None);
        assert_eq!(extension_of(None), None);
    }

    // -------- Expected type --------

    #[test]
    fn expected_type_is_compared() {
        let png = b"\x89PNG\r\n\x1A\n".to_vec();

        let report = detect_file_type(&FileTypeOptions {
            data: png.clone(),
            expected: Some(FileTypeKind::Png),
            ..FileTypeOptions::default()
        });
        assert_eq!(report.expected_matches, Some(true));

        let report = detect_file_type(&FileTypeOptions {
            data: png,
            expected: Some(FileTypeKind::Jpeg),
            ..FileTypeOptions::default()
        });
        assert_eq!(report.expected_matches, Some(false));

        let report = detect_file_type(&FileTypeOptions {
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
            expected: Some(FileTypeKind::Png),
            ..FileTypeOptions::default()
        });
        assert_eq!(report.expected_matches, Some(false));
    }

    // -------- Metadata --------

    #[test]
    fn every_kind_has_metadata() {
        for kind in ALL_KINDS {
            assert!(!kind.name().is_empty(), "{} has no name", kind.name());
            assert!(kind.mime().is_some(), "{} has no MIME type", kind.name());
            assert!(!kind.extensions().is_empty(), "{} has no extensions", kind.name());
        }
        assert_eq!(FileTypeKind::Unknown.name(), "unknown");
        assert_eq!(FileTypeKind::Unknown.mime(), None);
        assert!(FileTypeKind::Unknown.extensions().is_empty());
    }

    // -------- Cross-checks with the sample file generator --------

    #[test]
    fn generated_sample_files_are_detected() {
        let pdf = generate_sample_file(SampleKind::Pdf(PdfOptions::default()), None).expect("pdf generation");
        let report = detect(&pdf);
        assert_eq!(report.detected, FileTypeKind::Pdf);
        assert_eq!(report.confidence, FileConfidence::High);

        let png = generate_sample_file(SampleKind::Png(PngOptions::default()), None).expect("png generation");
        assert_eq!(detect(&png).detected, FileTypeKind::Png);

        let jpg = generate_sample_file(SampleKind::Jpg(JpgOptions::default()), None).expect("jpg generation");
        assert_eq!(detect(&jpg).detected, FileTypeKind::Jpeg);
    }

    #[test]
    fn tampered_sample_files_no_longer_match() {
        let zeroed =
            generate_sample_file(SampleKind::Png(PngOptions::default()), Some(TamperKind::MagicBytes)).expect("tamper");
        assert_ne!(detect(&zeroed).detected, FileTypeKind::Png);

        let filled =
            generate_sample_file(SampleKind::Png(PngOptions::default()), Some(TamperKind::ZeroFill)).expect("tamper");
        assert_ne!(detect(&filled).detected, FileTypeKind::Png);

        let text =
            generate_sample_file(SampleKind::Png(PngOptions::default()), Some(TamperKind::TextFill)).expect("tamper");
        assert_ne!(detect(&text).detected, FileTypeKind::Png);

        // Truncation keeps the leading signature intact: header-only
        // detection still reports the original type.
        let truncated =
            generate_sample_file(SampleKind::Png(PngOptions::default()), Some(TamperKind::Truncate)).expect("tamper");
        assert_eq!(detect(&truncated).detected, FileTypeKind::Png);
    }
}
