//! Sample file generator: valid PDF / PNG / JPEG files of a requested exact
//! size, optionally tampered, for testing upload endpoints (extension, MIME,
//! magic bytes, and size validation).

use std::fmt;

/// Largest file this generator will produce (2 GiB).
const MAX_FILE_SIZE: u64 = 2_000_000_000;

/// Errors that can occur when generating a sample file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleError {
    /// Invalid option values (colors, dimensions, oversized requests).
    Invalid(String),
    /// The requested exact size is not representable as a valid file.
    SizeTooSmall { requested: u64, minimum: u64 },
}

impl fmt::Display for SampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SampleError::Invalid(msg) => write!(f, "invalid sample file: {msg}"),
            SampleError::SizeTooSmall { requested, minimum } => {
                write!(f, "requested size {requested} bytes is below the minimum representable size {minimum} bytes")
            }
        }
    }
}

impl std::error::Error for SampleError {}

/// Corruption applied after generation, for negative server tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TamperKind {
    /// Overwrite the leading magic bytes with zeroes.
    MagicBytes,
    /// Keep only the first half of the file.
    Truncate,
    /// Replace every byte with a zero.
    ZeroFill,
    /// Replace every byte with the letter `a`.
    TextFill,
}

/// Kinds of sample files that can be generated.
#[derive(Debug)]
pub enum SampleKind {
    /// PDF document.
    Pdf(PdfOptions),
    /// PNG image.
    Png(PngOptions),
    /// JPEG image.
    Jpg(JpgOptions),
}

/// Options for PDF generation.
#[derive(Debug, Clone)]
pub struct PdfOptions {
    /// Exact byte count; `None` produces the smallest valid file.
    pub size: Option<u64>,
    /// Number of pages.
    pub pages: u32,
    /// Text drawn on every page; `None` uses a placeholder.
    pub text: Option<String>,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            size: None,
            pages: 1,
            text: None,
        }
    }
}

/// Options for PNG generation.
#[derive(Debug, Clone, Default)]
pub struct PngOptions {
    /// Exact byte count; `None` produces the smallest valid image.
    pub size: Option<u64>,
    /// Image width; `None` picks dimensions that fit the requested size.
    pub width: Option<u32>,
    /// Image height; `None` picks dimensions that fit the requested size.
    pub height: Option<u32>,
    /// Pixel color as `RRGGBB` hex or a named color; `None` uses gray.
    pub color: Option<String>,
}

/// Options for JPEG generation.
#[derive(Debug, Clone, Default)]
pub struct JpgOptions {
    /// Exact byte count; `None` produces the smallest valid image.
    pub size: Option<u64>,
}

/// Generate a sample file according to `kind`, optionally corrupting the
/// result with `tamper` (applied after sizing).
pub fn generate_sample_file(kind: SampleKind, tamper: Option<TamperKind>) -> Result<Vec<u8>, SampleError> {
    let bytes = match kind {
        SampleKind::Pdf(options) => build_pdf(&options)?,
        SampleKind::Png(options) => build_png(&options)?,
        SampleKind::Jpg(options) => build_jpg(&options)?,
    };
    Ok(apply_tamper(bytes, tamper))
}

// -------- Shared helpers --------

fn check_size(target: u64) -> Result<(), SampleError> {
    if target > MAX_FILE_SIZE {
        return Err(SampleError::Invalid(format!(
            "requested size exceeds the maximum supported size of {MAX_FILE_SIZE} bytes"
        )));
    }
    Ok(())
}

/// CRC-32 (IEEE 802.3, reflected polynomial 0xEDB88320), as used by PNG chunks.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Adler-32 checksum of `data`, as used by the zlib trailer (exponent 65521).
fn adler32(data: &[u8]) -> u32 {
    let mut low: u32 = 1;
    let mut high: u32 = 0;
    for &byte in data {
        low = (low + u32::from(byte)) % 65521;
        high = (high + low) % 65521;
    }
    (high << 16) | low
}

/// Wrap `data` in a zlib stream (`0x78 0x01` header, stored blocks, Adler-32).
///
/// Stored (uncompressed) deflate blocks are spec-valid, so no compression
/// dependency is needed; PNG decoders accept them everywhere.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 16 + data.len() / 65535 * 5);
    out.extend_from_slice(&[0x78, 0x01]);
    let mut remaining = data;
    loop {
        let take = remaining.len().min(65535);
        let (block, rest) = remaining.split_at(take);
        let final_bit = if rest.is_empty() { 1u8 } else { 0u8 };
        out.push(final_bit); // BFINAL + BTYPE 00 (stored)
        out.extend_from_slice(&(take as u16).to_le_bytes());
        out.extend_from_slice(&(!(take as u16)).to_le_bytes());
        out.extend_from_slice(block);
        if rest.is_empty() {
            break;
        }
        remaining = rest;
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn apply_tamper(bytes: Vec<u8>, tamper: Option<TamperKind>) -> Vec<u8> {
    let Some(kind) = tamper else {
        return bytes;
    };
    match kind {
        TamperKind::MagicBytes => {
            let mut out = bytes;
            let count = out.len().min(4);
            out[..count].fill(0);
            out
        }
        TamperKind::Truncate => bytes[..bytes.len() / 2].to_vec(),
        TamperKind::ZeroFill => vec![0; bytes.len()],
        TamperKind::TextFill => vec![b'a'; bytes.len()],
    }
}

// -------- PDF --------

const PAGE_WIDTH: u32 = 612;
const PAGE_HEIGHT: u32 = 792;
const MAX_PAGES: u32 = 1000;

fn build_pdf(options: &PdfOptions) -> Result<Vec<u8>, SampleError> {
    let pages = options.pages.max(1);
    if pages > MAX_PAGES {
        return Err(SampleError::Invalid(format!("pages must be between 1 and {MAX_PAGES}")));
    }
    let base = assemble_pdf(pages, options.text.as_deref(), 0);
    let minimum = base.len() as u64;
    let Some(target) = options.size else {
        return Ok(base);
    };
    check_size(target)?;
    if target < minimum {
        return Err(SampleError::SizeTooSmall {
            requested: target,
            minimum,
        });
    }
    // Padding is written into the last content stream; `/Length` digit-count
    // changes perturb the total, so iterate until the size converges exactly.
    let mut padding = (target - minimum) as usize;
    let mut bytes = assemble_pdf(pages, options.text.as_deref(), padding);
    for _ in 0..32 {
        let current = bytes.len() as i64;
        let wanted = target as i64;
        if current == wanted {
            return Ok(bytes);
        }
        padding = (padding as i64 + wanted - current).max(1) as usize;
        bytes = assemble_pdf(pages, options.text.as_deref(), padding);
    }
    Err(SampleError::Invalid(String::from("failed to reach the requested size (internal error)")))
}

fn assemble_pdf(pages: u32, text: Option<&str>, padding: usize) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");

    let mut offsets: Vec<u32> = Vec::with_capacity(3 + 2 * pages as usize);
    let font_number = 3 + pages;

    push_pdf_object(&mut out, &mut offsets, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    let kids: String = (0..pages).map(|page| format!("{} 0 R ", 3 + page)).collect();
    push_pdf_object(&mut out, &mut offsets, 2, &format!("<< /Type /Pages /Kids [{kids}] /Count {pages} >>"));
    for page in 0..pages {
        let contents = contents_number(page, pages);
        push_pdf_object(
            &mut out,
            &mut offsets,
            3 + page,
            &format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] \
                 /Resources << /Font << /F1 {font_number} 0 R >> >> /Contents {contents} 0 R >>"
            ),
        );
    }
    push_pdf_object(&mut out, &mut offsets, font_number, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>");
    for page in 0..pages {
        let content = content_stream(text, if page == pages - 1 { padding } else { 0 });
        // The length and startxref offsets are zero-padded to a fixed width so
        // that padding bytes change the file size 1:1, keeping exact sizing
        // trivial and unambiguous (leading zeros are valid PDF integers).
        let body = format!("<< /Length {:010} >>\nstream\n{content}\nendstream", content.len());
        push_pdf_object(&mut out, &mut offsets, contents_number(page, pages), &body);
    }

    let xref_offset = out.len() as u32;
    out.extend_from_slice(b"xref\n");
    out.extend_from_slice(format!("0 {}\r\n", offsets.len()).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(b"trailer\r\n");
    out.extend_from_slice(format!("<< /Size {} /Root 1 0 R >>\r\n", offsets.len()).as_bytes());
    out.extend_from_slice(format!("startxref\r\n{xref_offset:010}\r\n%%EOF").as_bytes());
    out
}

fn push_pdf_object(out: &mut Vec<u8>, offsets: &mut Vec<u32>, number: u32, body: &str) {
    offsets.push(out.len() as u32);
    out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
    out.extend_from_slice(body.as_bytes());
    out.extend_from_slice(b"\nendobj\n");
}

/// Object number of the content stream for `page` (0-based), given `pages` total.
fn contents_number(page: u32, pages: u32) -> u32 {
    4 + pages + page
}

/// Build the content stream body: a text operator plus up to `padding` bytes
/// of trailing padding (whitespace or `%` comments, both legal in PDF streams).
fn content_stream(text: Option<&str>, padding: usize) -> String {
    let default = "oxide sample file";
    let sanitized: String = text
        .unwrap_or(default)
        .chars()
        .map(|ch| if ch.is_ascii_graphic() || ch == ' ' { ch } else { ' ' })
        .collect();
    let escaped = sanitized.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
    let mut content = String::new();
    content.push_str("BT /F1 24 Tf 72 720 Td (");
    content.push_str(&escaped);
    content.push_str(") Tj ET\n");
    match padding {
        0 => {}
        1 => content.push(' '),
        2 => content.push_str("%\n"),
        _ => {
            content.push_str("% ");
            content.push_str(&"x".repeat(padding - 3));
            content.push('\n');
        }
    }
    content
}

// -------- PNG --------

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// Bytes outside the raster: signature + IHDR chunk + IEND chunk + IDAT chunk
/// overhead (length/type/CRC) + zlib header + Adler-32 trailer.
const MIN_OVERHEAD: u64 = 63;

fn build_png(options: &PngOptions) -> Result<Vec<u8>, SampleError> {
    let rgb = parse_color(options.color.as_deref())?;
    let explicit = options.width.is_some() || options.height.is_some();
    let (requested_width, requested_height) = (options.width.unwrap_or(1).max(1), options.height.unwrap_or(1).max(1));
    let minimal = compose_png(1, 1, rgb, 0);
    let minimum = minimal.len() as u64;
    let Some(target) = options.size else {
        // Without a size, honor explicit dimensions or fall back to 1x1.
        return if explicit {
            Ok(compose_png(requested_width, requested_height, rgb, 0))
        } else {
            Ok(minimal)
        };
    };
    check_size(target)?;
    if target < minimum {
        return Err(SampleError::SizeTooSmall {
            requested: target,
            minimum,
        });
    }

    let (mut width, mut height) = if explicit {
        (requested_width, requested_height)
    } else {
        auto_dimensions(target)
    };
    let data_len = u128::from(height) * (1 + 3 * u128::from(width));
    if data_len + 128 > u128::from(MAX_FILE_SIZE) {
        return Err(SampleError::Invalid(String::from("image dimensions are too large for the requested size")));
    }

    // The raster occupies row_bytes per row and the last chunk overhead is
    // fixed; shrink the height until a tEXt padding chunk closes the gap.
    for _ in 0..32 {
        let len = compose_png(width, height, rgb, 0).len() as u64;
        let delta = target as i64 - len as i64;
        if delta >= 20 {
            return Ok(compose_png(width, height, rgb, (delta - 20) as usize));
        }
        if height > 1 {
            height -= 1;
            continue;
        }
        if !explicit && width > 1 {
            width = 1;
            height = strip_height(target);
            continue;
        }
        let minimum = if delta >= 0 { len + 20 } else { len };
        return Err(SampleError::SizeTooSmall {
            requested: target,
            minimum,
        });
    }
    Err(SampleError::Invalid(String::from("failed to reach the requested size (internal error)")))
}

/// Pick roughly square dimensions whose raster fits inside `target`, leaving
/// room for the final tEXt padding chunk.
fn auto_dimensions(target: u64) -> (u32, u32) {
    let pixel_budget = target.saturating_sub(MIN_OVERHEAD) / 3;
    let width = ((pixel_budget as f64).sqrt() as u64).clamp(1, 2048);
    let row_bytes = 1 + 3 * width;
    let height = ((target - MIN_OVERHEAD) / row_bytes).max(1);
    (width as u32, height as u32)
}

/// Row count for a 1-pixel-wide raster that fits inside `target`.
fn strip_height(target: u64) -> u32 {
    ((target - MIN_OVERHEAD) / 4).max(1) as u32
}

/// Assemble a PNG with the given raster and `text_fill` bytes of comment text.
fn compose_png(width: u32, height: u32, rgb: [u8; 3], text_fill: usize) -> Vec<u8> {
    let row_bytes = 1 + 3 * width as usize;
    let mut row = Vec::with_capacity(row_bytes);
    row.push(0); // filter type 0 (none)
    for _ in 0..width {
        row.extend_from_slice(&rgb);
    }

    let mut raster = Vec::with_capacity(row_bytes * height as usize);
    for _ in 0..height {
        raster.extend_from_slice(&row);
    }

    let mut out = Vec::with_capacity(raster.len() + 128);
    out.extend_from_slice(&PNG_SIGNATURE);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit RGB, no interlace
    push_png_chunk(&mut out, b"IHDR", &ihdr);

    let compressed = zlib_stored(&raster);
    push_png_chunk(&mut out, b"IDAT", &compressed);

    if text_fill > 0 {
        let mut text = b"Comment".to_vec();
        text.push(0);
        text.extend(std::iter::repeat_n(b'x', text_fill));
        push_png_chunk(&mut out, b"tEXt", &text);
    }

    push_png_chunk(&mut out, b"IEND", &[]);
    out
}

fn push_png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// Parse `RRGGBB` hex or a named color into RGB bytes.
fn parse_color(color: Option<&str>) -> Result<[u8; 3], SampleError> {
    let invalid =
        |value: &str| SampleError::Invalid(format!("invalid color \"{value}\", expected RRGGBB hex or a named color"));
    match color {
        Some(value) => {
            let hex = value.trim_start_matches('#');
            if hex.len() == 6 {
                match u32::from_str_radix(hex, 16) {
                    Ok(rgb) => return Ok([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8]),
                    Err(_) => return Err(invalid(value)),
                }
            }
            match value.to_ascii_lowercase().as_str() {
                "red" => Ok([0xFF, 0x00, 0x00]),
                "green" => Ok([0x00, 0x80, 0x00]),
                "blue" => Ok([0x00, 0x00, 0xFF]),
                "black" => Ok([0x00, 0x00, 0x00]),
                "white" => Ok([0xFF, 0xFF, 0xFF]),
                "gray" | "grey" => Ok([0x80, 0x80, 0x80]),
                _ => Err(invalid(value)),
            }
        }
        None => Ok([0x80, 0x80, 0x80]),
    }
}

// -------- JPEG --------

/// Minimal baseline JPEG: a 1x1 grayscale image with a two-symbol Huffman
/// table per class. The scan data is DC delta 0 then EOB, i.e. bits `00`
/// padded with ones to `0x3F`, which decodes to a single black pixel.
#[rustfmt::skip]
const MINIMAL_JPEG: [u8; 161] = [
    0xFF, 0xD8,                                                                 // SOI
    0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00,
    0x00, 0x01, 0x00, 0x01, 0x00, 0x00,                                         // APP0 (JFIF 1.1)
    0xFF, 0xDB, 0x00, 0x43, 0x00,                                               // DQT, 8-bit luminance table
    0x10, 0x0B, 0x0A, 0x10, 0x18, 0x28, 0x33, 0x3D, 0x0C, 0x0C, 0x0E, 0x13,
    0x1A, 0x3A, 0x3C, 0x37, 0x0E, 0x0D, 0x10, 0x18, 0x28, 0x39, 0x45, 0x38,
    0x0E, 0x11, 0x16, 0x1D, 0x33, 0x57, 0x50, 0x3E, 0x12, 0x16, 0x25, 0x38,
    0x44, 0x6D, 0x67, 0x4D, 0x18, 0x23, 0x37, 0x40, 0x51, 0x68, 0x71, 0x5C,
    0x31, 0x40, 0x4E, 0x57, 0x67, 0x79, 0x78, 0x65, 0x48, 0x5C, 0x5F, 0x62,
    0x70, 0x64, 0x67, 0x63,                                                     // 64 quantization values
    0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11,
    0x00,                                                                       // SOF0: 8-bit, 1x1, one component
    0xFF, 0xC4, 0x00, 0x15, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,         // DHT, DC
    0xFF, 0xC4, 0x00, 0x15, 0x10, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,         // DHT, AC
    0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00,               // SOS
    0x3F,                                                                       // entropy: DC 0 + EOB
    0xFF, 0xD9,                                                                 // EOI
];

fn build_jpg(options: &JpgOptions) -> Result<Vec<u8>, SampleError> {
    let Some(target) = options.size else {
        return Ok(MINIMAL_JPEG.to_vec());
    };
    check_size(target)?;
    let minimum = MINIMAL_JPEG.len() as u64;
    if target < minimum {
        return Err(SampleError::SizeTooSmall {
            requested: target,
            minimum,
        });
    }
    let padding = target - minimum;
    if padding == 0 {
        return Ok(MINIMAL_JPEG.to_vec());
    }
    // Pad with COM (comment) segments. A segment with `content` payload bytes
    // occupies content + 4 bytes total (marker + length field + payload) and
    // caps at 65537 bytes, so a remaining gap of 1-3 bytes is unrepresentable.
    let remainder = padding % 65537;
    if (1..=3).contains(&remainder) {
        let reachable = target + (4 - remainder);
        return Err(SampleError::SizeTooSmall {
            requested: target,
            minimum: reachable,
        });
    }
    let mut out = Vec::with_capacity(target as usize);
    out.extend_from_slice(&MINIMAL_JPEG[..20]); // SOI + APP0, comments go after
    append_comment_segments(&mut out, padding as usize);
    out.extend_from_slice(&MINIMAL_JPEG[20..]);
    Ok(out)
}

/// Append chained COM segments whose total byte count is exactly `padding`.
fn append_comment_segments(out: &mut Vec<u8>, padding: usize) {
    let full_segments = padding / 65537;
    let remainder = padding % 65537;
    for _ in 0..full_segments {
        push_comment_segment(out, 65533);
    }
    if remainder != 0 {
        push_comment_segment(out, remainder - 4);
    }
}

fn push_comment_segment(out: &mut Vec<u8>, content_len: usize) {
    out.extend_from_slice(&[0xFF, 0xFE]);
    out.extend_from_slice(&((content_len + 2) as u16).to_be_bytes());
    out.extend(std::iter::repeat_n(b'x', content_len));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pdf_bytes(options: PdfOptions) -> Vec<u8> {
        generate_sample_file(SampleKind::Pdf(options), None).expect("pdf generation failed")
    }

    fn png_bytes(options: PngOptions) -> Vec<u8> {
        generate_sample_file(SampleKind::Png(options), None).expect("png generation failed")
    }

    fn jpg_bytes(options: JpgOptions) -> Vec<u8> {
        generate_sample_file(SampleKind::Jpg(options), None).expect("jpg generation failed")
    }

    // -------- Checksums --------

    #[test]
    fn crc32_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn adler32_known_vector() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn zlib_stored_roundtrip_payload() {
        let data: Vec<u8> = (0..100_000u32).map(|value| (value % 251) as u8).collect();
        let zlib = zlib_stored(&data);
        assert_eq!(&zlib[..2], &[0x78, 0x01]);
        let inflated = inflate_stored(&zlib);
        assert_eq!(inflated, data);
    }

    fn inflate_stored(zlib: &[u8]) -> Vec<u8> {
        assert_eq!(&zlib[..2], &[0x78, 0x01], "unexpected zlib header");
        let mut out = Vec::new();
        let mut offset = 2;
        loop {
            let header = zlib[offset];
            offset += 1;
            assert_eq!(header & 0b110, 0b000, "expected a stored deflate block");
            let len = u16::from_le_bytes([zlib[offset], zlib[offset + 1]]) as usize;
            let nlen = u16::from_le_bytes([zlib[offset + 2], zlib[offset + 3]]);
            offset += 4;
            assert_eq!(!(len as u16), nlen, "NLEN mismatch");
            out.extend_from_slice(&zlib[offset..offset + len]);
            offset += len;
            if header & 1 == 1 {
                break;
            }
        }
        let adler = u32::from_be_bytes([zlib[offset], zlib[offset + 1], zlib[offset + 2], zlib[offset + 3]]);
        assert_eq!(adler, adler32(&out), "Adler-32 mismatch");
        out
    }

    // -------- PDF --------

    #[test]
    fn pdf_minimal_is_valid() {
        let bytes = pdf_bytes(PdfOptions::default());
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.ends_with(b"%%EOF"));
        assert!(bytes.len() < 1_000);
    }

    #[test]
    fn pdf_5kb_is_exact_size() {
        let bytes = pdf_bytes(PdfOptions {
            size: Some(5_000),
            ..PdfOptions::default()
        });
        assert_eq!(bytes.len() as u64, 5_000);
        assert!(bytes.starts_with(b"%PDF-1.4"));
    }

    #[test]
    fn pdf_1mb_multipage_is_exact_size() {
        let bytes = pdf_bytes(PdfOptions {
            size: Some(1_000_000),
            pages: 2,
            ..PdfOptions::default()
        });
        assert_eq!(bytes.len() as u64, 1_000_000);
        assert_eq!(count_occurrences(&bytes, " /Type /Page "), 2);
    }

    #[test]
    fn pdf_is_exact_for_every_target_in_range() {
        let minimum = pdf_bytes(PdfOptions::default()).len() as u64;
        for target in minimum..minimum + 64 {
            let options = PdfOptions {
                size: Some(target),
                ..PdfOptions::default()
            };
            let bytes = pdf_bytes(options);
            assert_eq!(bytes.len() as u64, target, "size drift at {target}");
        }
    }

    #[test]
    fn pdf_size_below_minimum_errors() {
        let err = generate_sample_file(
            SampleKind::Pdf(PdfOptions {
                size: Some(1),
                ..Default::default()
            }),
            None,
        )
        .unwrap_err();
        assert!(matches!(err, SampleError::SizeTooSmall { requested: 1, .. }));
        assert!(err.to_string().contains("below the minimum"));
    }

    #[test]
    fn pdf_xref_offsets_are_valid() {
        let bytes = pdf_bytes(PdfOptions {
            pages: 3,
            ..PdfOptions::default()
        });
        assert_pdf_xref_valid(&bytes);
    }

    #[test]
    fn pdf_contains_requested_text() {
        let bytes = pdf_bytes(PdfOptions {
            text: Some("invoice #1".into()),
            ..PdfOptions::default()
        });
        assert!(bytes.windows(b"invoice #1".len()).any(|w| w == b"invoice #1"));
        assert!(bytes.windows(b"Tj".len()).any(|w| w == b"Tj"));
    }

    #[test]
    fn pdf_escapes_special_characters() {
        let bytes = pdf_bytes(PdfOptions {
            text: Some("a(b)\\c".into()),
            ..PdfOptions::default()
        });
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("a\\(b\\)\\\\c"));
    }

    #[test]
    fn pdf_pages_zero_uses_single_page() {
        let bytes = pdf_bytes(PdfOptions {
            pages: 0,
            ..PdfOptions::default()
        });
        assert_eq!(count_occurrences(&bytes, " /Type /Page "), 1);
    }

    #[test]
    fn pdf_deterministic() {
        let options = PdfOptions {
            size: Some(5_000),
            ..PdfOptions::default()
        };
        assert_eq!(pdf_bytes(options.clone()), pdf_bytes(options));
    }

    fn count_occurrences(haystack: &[u8], needle: &str) -> usize {
        haystack
            .windows(needle.len())
            .filter(|window| window == &needle.as_bytes())
            .count()
    }

    /// Parse the xref table and verify every entry points at its own object
    /// header and that `startxref` locates the table.
    fn assert_pdf_xref_valid(bytes: &[u8]) {
        let target = std::str::from_utf8(bytes).expect("pdf should be ASCII");
        let xref_offset = parse_after(target, "startxref\r\n");
        assert!(xref_offset < bytes.len());
        assert!(bytes[xref_offset..].starts_with(b"xref"), "startxref points at a non-xref");

        let count_line_start = xref_offset + "xref\n".len();
        let count_line_end = count_line_start
            + bytes[count_line_start..]
                .windows(2)
                .position(|window| window == b"\r\n")
                .expect("count line without EOL")
            + 2;
        let line = &target[count_line_start..count_line_end];
        let fields: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(fields[0], "0");
        let count = fields[1].parse::<usize>().unwrap();
        let mut offset = count_line_end;
        // Entry 0 is the mandatory free-object entry.
        let free_entry = &bytes[offset..offset + 20];
        assert_eq!(free_entry, b"0000000000 65535 f\r\n");
        offset += 20;
        for object_number in 1..count {
            let entry = &bytes[offset..offset + 20];
            assert_eq!(entry.len(), 20, "xref entry must be 20 bytes");
            let object_offset: usize = std::str::from_utf8(&entry[..10]).unwrap().trim().parse().unwrap();
            let header = format!("{object_number} 0 obj\n");
            assert!(
                bytes[object_offset..].starts_with(header.as_bytes()),
                "xref entry {object_number} points at {object_offset}, expected object"
            );
            offset += 20;
        }
    }

    /// Parse the number that follows `anchor` in `target`.
    fn parse_after(target: &str, anchor: &str) -> usize {
        let start = target.find(anchor).expect("anchor not found") + anchor.len();
        let rest = &target[start..];
        let end = rest.find('\n').unwrap_or(rest.len());
        rest[..end].trim().parse().unwrap()
    }

    // -------- PNG --------

    /// Walk the PNG chunk stream, verifying lengths and CRCs, and return
    /// (kind, data) pairs in file order.
    fn png_chunks(bytes: &[u8]) -> Vec<(&[u8], &[u8])> {
        assert_eq!(&bytes[..8], &PNG_SIGNATURE, "bad PNG signature");
        let mut offset = 8;
        let mut chunks = Vec::new();
        while offset < bytes.len() {
            assert!(offset + 12 <= bytes.len(), "truncated chunk header");
            let len =
                u32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]) as usize;
            assert!(offset + 12 + len <= bytes.len(), "chunk overruns the file");
            let kind = &bytes[offset + 4..offset + 8];
            let data = &bytes[offset + 8..offset + 8 + len];
            let stored_crc = u32::from_be_bytes([
                bytes[offset + 8 + len],
                bytes[offset + 9 + len],
                bytes[offset + 10 + len],
                bytes[offset + 11 + len],
            ]);
            let mut crc_input = Vec::with_capacity(4 + len);
            crc_input.extend_from_slice(kind);
            crc_input.extend_from_slice(data);
            assert_eq!(stored_crc, crc32(&crc_input), "bad CRC on {:?}", kind);
            chunks.push((kind, data));
            offset += 12 + len;
        }
        assert_eq!(chunks.last().unwrap().0, b"IEND", "file must end with IEND");
        chunks
    }

    fn png_ihdr(chunks: &[(&[u8], &[u8])]) -> (u32, u32) {
        let (kind, data) = chunks.iter().find(|(kind, _)| *kind == b"IHDR").expect("no IHDR");
        assert_eq!(*kind, b"IHDR");
        let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        assert_eq!(&data[8..], &[8, 2, 0, 0, 0], "unexpected IHDR fields");
        (width, height)
    }

    #[test]
    fn png_minimal_is_valid() {
        let bytes = png_bytes(PngOptions::default());
        assert_eq!(bytes.len(), 72);
        let chunks = png_chunks(&bytes);
        assert_eq!(png_ihdr(&chunks), (1, 1));
    }

    #[test]
    fn png_5mb_is_exact_size() {
        let bytes = png_bytes(PngOptions {
            size: Some(5_000_000),
            ..PngOptions::default()
        });
        assert_eq!(bytes.len() as u64, 5_000_000);
        let (width, height) = png_ihdr(&png_chunks(&bytes));
        assert!(width > 1 && height > 1, "expected raster dimensions, got {width}x{height}");
    }

    #[test]
    fn png_5kb_is_exact_size() {
        let bytes = png_bytes(PngOptions {
            size: Some(5_000),
            ..PngOptions::default()
        });
        assert_eq!(bytes.len() as u64, 5_000);
    }

    #[test]
    fn png_explicit_dimensions_pad_to_exact_size() {
        let options = PngOptions {
            size: Some(10_000),
            width: Some(32),
            height: Some(16),
            ..PngOptions::default()
        };
        let bytes = png_bytes(options);
        assert_eq!(bytes.len() as u64, 10_000);
        let chunks = png_chunks(&bytes);
        assert_eq!(png_ihdr(&chunks), (32, 16));
        let (kind, idat) = chunks.iter().find(|(kind, _)| *kind == b"IDAT").expect("no IDAT");
        assert_eq!(*kind, b"IDAT");
        let raster = inflate_stored(idat);
        assert_eq!(raster.len(), 16 * (1 + 3 * 32));
        assert!(chunks.iter().any(|(kind, _)| *kind == b"tEXt"), "expected tEXt padding chunk");
    }

    #[test]
    fn png_hex_color_appears_in_raster() {
        let options = PngOptions {
            width: Some(2),
            height: Some(1),
            color: Some("#ff0000".into()),
            ..PngOptions::default()
        };
        let bytes = png_bytes(options);
        let chunks = png_chunks(&bytes);
        let (_, idat) = chunks.iter().find(|(kind, _)| *kind == b"IDAT").expect("no IDAT");
        let raster = inflate_stored(idat);
        assert_eq!(&raster[..7], &[0, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00]);
    }

    #[test]
    fn png_explicit_dims_size_in_gap_errors() {
        let options = PngOptions {
            size: Some(80), // 72-byte minimal image; smallest tEXt pad lands at 92
            width: Some(1),
            height: Some(1),
            ..PngOptions::default()
        };
        let err = generate_sample_file(SampleKind::Png(options), None).unwrap_err();
        assert!(matches!(
            err,
            SampleError::SizeTooSmall {
                requested: 80,
                minimum: 92
            }
        ));
    }

    #[test]
    fn png_size_below_minimum_errors() {
        let options = PngOptions {
            size: Some(10),
            ..PngOptions::default()
        };
        let err = generate_sample_file(SampleKind::Png(options), None).unwrap_err();
        assert!(err.to_string().contains("below the minimum"));
    }

    #[test]
    fn png_invalid_color_errors() {
        let options = PngOptions {
            color: Some("chartreuse".into()),
            ..PngOptions::default()
        };
        let err = generate_sample_file(SampleKind::Png(options), None).unwrap_err();
        assert!(err.to_string().contains("color"));
    }

    #[test]
    fn png_absurd_dimensions_error() {
        let options = PngOptions {
            size: Some(5_000_000),
            width: Some(u32::MAX),
            height: Some(u32::MAX),
            ..PngOptions::default()
        };
        let err = generate_sample_file(SampleKind::Png(options), None).unwrap_err();
        assert!(err.to_string().contains("dimensions"));
    }

    #[test]
    fn png_deterministic() {
        let options = PngOptions {
            size: Some(5_000),
            ..PngOptions::default()
        };
        assert_eq!(png_bytes(options.clone()), png_bytes(options));
    }

    // -------- JPEG --------

    /// Walk top-level markers, verifying segment lengths, DHT symbol counts,
    /// and byte-stuffed entropy data, and assert the file ends at EOI.
    fn assert_jpeg_structure(bytes: &[u8]) {
        assert_eq!(&bytes[..2], &[0xFF, 0xD8], "missing SOI");
        let mut offset = 2;
        loop {
            assert!(offset + 2 <= bytes.len(), "truncated marker at {offset}");
            assert_eq!(bytes[offset], 0xFF, "expected marker at {offset}");
            let marker = bytes[offset + 1];
            offset += 2;
            match marker {
                0xD9 => {
                    assert_eq!(offset, bytes.len(), "data after EOI");
                    return;
                }
                0xDA => {
                    let mut cursor = offset;
                    while cursor < bytes.len() - 1 {
                        if bytes[cursor] == 0xFF {
                            let next = bytes[cursor + 1];
                            assert!(
                                next == 0x00 || next == 0xD9 || (0xD0..=0xD7).contains(&next),
                                "unstuffed 0xFF at {cursor}"
                            );
                            if next == 0xD9 {
                                break;
                            }
                        }
                        cursor += 1;
                    }
                    assert_eq!(&bytes[cursor..cursor + 2], &[0xFF, 0xD9], "missing EOI");
                    return;
                }
                0xC4 => {
                    let length = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) as usize;
                    let counts = &bytes[offset + 3..offset + 19];
                    let symbol_total: usize = counts.iter().map(|&count| usize::from(count)).sum();
                    assert_eq!(symbol_total, length - 2 - 1 - 16, "DHT symbol count mismatch");
                    offset += length;
                }
                _ => {
                    let length = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) as usize;
                    assert!(length >= 2, "segment with impossible length");
                    offset += length;
                }
            }
        }
    }

    #[test]
    fn jpg_minimal_is_valid() {
        let bytes = jpg_bytes(JpgOptions::default());
        assert_eq!(bytes.len(), 161);
        assert_jpeg_structure(&bytes);
    }

    #[test]
    fn jpg_5mb_is_exact_size() {
        let bytes = jpg_bytes(JpgOptions { size: Some(5_000_000) });
        assert_eq!(bytes.len() as u64, 5_000_000);
        assert_eq!(&bytes[..2], &[0xFF, 0xD8]);
        assert_eq!(&bytes[bytes.len() - 2..], &[0xFF, 0xD9]);
    }

    #[test]
    fn jpg_exact_across_comment_boundaries() {
        let minimum = 161;
        let exact_sizes = [
            minimum + 4,
            minimum + 65534,
            minimum + 65535,
            minimum + 65536,
            minimum + 65537,
            minimum + 65541,
        ];
        for target in exact_sizes {
            let bytes = jpg_bytes(JpgOptions { size: Some(target) });
            assert_eq!(bytes.len() as u64, target);
            assert_jpeg_structure(&bytes);
        }
    }

    #[test]
    fn jpg_unrepresentable_padding_errors() {
        let minimum = 161u64;
        for gap in 1..=3 {
            let target = minimum + gap;
            let err = generate_sample_file(SampleKind::Jpg(JpgOptions { size: Some(target) }), None).unwrap_err();
            assert_eq!(
                err,
                SampleError::SizeTooSmall {
                    requested: target,
                    minimum: target + 4 - gap
                }
            );
        }
        let target = minimum + 65538; // one byte past a maximum-size comment segment
        let err = generate_sample_file(SampleKind::Jpg(JpgOptions { size: Some(target) }), None).unwrap_err();
        assert_eq!(
            err,
            SampleError::SizeTooSmall {
                requested: target,
                minimum: target + 3
            }
        );
    }

    #[test]
    fn jpg_size_below_minimum_errors() {
        let err = generate_sample_file(SampleKind::Jpg(JpgOptions { size: Some(10) }), None).unwrap_err();
        assert!(matches!(
            err,
            SampleError::SizeTooSmall {
                requested: 10,
                minimum: 161
            }
        ));
    }

    #[test]
    fn jpg_deterministic() {
        let options = JpgOptions { size: Some(5_000) };
        assert_eq!(jpg_bytes(options.clone()), jpg_bytes(options));
    }

    // -------- Tampering --------

    #[test]
    fn tamper_magic_zeroes_first_four_bytes() {
        let mut bytes = pdf_bytes(PdfOptions {
            size: Some(5_000),
            ..PdfOptions::default()
        });
        let len = bytes.len();
        bytes = apply_tamper(bytes, Some(TamperKind::MagicBytes));
        assert_eq!(bytes.len(), len);
        assert_eq!(&bytes[..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn tamper_truncate_halves_file() {
        let bytes = pdf_bytes(PdfOptions {
            size: Some(5_000),
            ..PdfOptions::default()
        });
        let half = bytes.len() / 2;
        let bytes = apply_tamper(bytes, Some(TamperKind::Truncate));
        assert_eq!(bytes.len(), half);
    }

    #[test]
    fn tamper_zero_fill() {
        let bytes = pdf_bytes(PdfOptions {
            size: Some(5_000),
            ..PdfOptions::default()
        });
        let len = bytes.len();
        let bytes = apply_tamper(bytes, Some(TamperKind::ZeroFill));
        assert_eq!(bytes.len(), len);
        assert!(bytes.iter().all(|&byte| byte == 0));
    }

    #[test]
    fn tamper_text_fill() {
        let bytes = jpg_bytes(JpgOptions { size: Some(5_000) });
        let len = bytes.len();
        let bytes = apply_tamper(bytes, Some(TamperKind::TextFill));
        assert_eq!(bytes.len(), len);
        assert!(bytes.iter().all(|&byte| byte == b'a'));
    }

    #[test]
    fn tamper_via_public_entry_point() {
        let bytes = generate_sample_file(
            SampleKind::Pdf(PdfOptions {
                size: Some(5_000),
                ..Default::default()
            }),
            Some(TamperKind::Truncate),
        )
        .unwrap();
        assert_eq!(bytes.len(), 2_500);
    }

    #[test]
    fn all_kinds_start_with_expected_magic() {
        let pdf = pdf_bytes(PdfOptions::default());
        let png = png_bytes(PngOptions::default());
        let jpg = jpg_bytes(JpgOptions::default());
        assert!(pdf.starts_with(b"%PDF"));
        assert!(png.starts_with(&PNG_SIGNATURE));
        assert_eq!(&jpg[..2], &[0xFF, 0xD8]);
    }
}
