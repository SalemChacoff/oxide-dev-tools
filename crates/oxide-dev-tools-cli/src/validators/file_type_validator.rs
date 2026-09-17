use std::fs;
use std::io::Read;
use std::path::Path;

use clap::{Args, ValueEnum};
use oxide_dev_tools_core::*;

use crate::error::{GenericError, ValidError};

/// `oxide validate file <PATH> [options]` — content-based file type detection
#[derive(Args)]
#[command(
    after_help = "Examples:\n  oxide validate file photo.png\n  oxide validate file archive.zip --verbose\n  oxide validate file upload.bin --expected png\n  oxide validate file image.jpg --strict\n  cat data.bin | oxide validate file -"
)]
pub struct FileArgs {
    /// File path to detect, or "-" to read from stdin
    pub input: String,

    /// Require the file to be detected as this type
    #[arg(long, value_enum)]
    pub expected: Option<ExpectedFileType>,

    /// Disable the warning when the detected type disagrees with the file extension
    #[arg(long)]
    pub no_extension_check: bool,

    /// Treat an extension mismatch as an error
    #[arg(long)]
    pub strict: bool,

    /// Print the full detection report
    #[arg(long)]
    pub verbose: bool,
}

/// Expected type choices for the `--expected` flag.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ExpectedFileType {
    /// Require a PDF document.
    Pdf,
    /// Require a Rich Text Format document.
    Rtf,
    /// Require an OOXML Word document.
    Docx,
    /// Require an OOXML spreadsheet.
    Xlsx,
    /// Require an OOXML presentation.
    Pptx,
    /// Require an OpenDocument text.
    Odt,
    /// Require an OpenDocument spreadsheet.
    Ods,
    /// Require an OpenDocument presentation.
    Odp,
    /// Require an EPUB book.
    Epub,
    /// Require a Java archive.
    Jar,
    /// Require a PNG image.
    Png,
    /// Require a JPEG image.
    Jpeg,
    /// Require a GIF image.
    Gif,
    /// Require a Windows bitmap.
    Bmp,
    /// Require a WebP image.
    #[value(name = "webp")]
    WebP,
    /// Require a TIFF image.
    Tiff,
    /// Require an ICO icon.
    Ico,
    /// Require an SVG image.
    Svg,
    /// Require a Photoshop document.
    Psd,
    /// Require MP3 audio.
    Mp3,
    /// Require WAVE audio.
    Wav,
    /// Require FLAC audio.
    Flac,
    /// Require Ogg audio.
    Ogg,
    /// Require AAC (ADTS) audio.
    Aac,
    /// Require MPEG-4 audio.
    #[value(name = "m4a")]
    M4a,
    /// Require MIDI audio.
    Midi,
    /// Require MPEG-4 video.
    Mp4,
    /// Require WebM or Matroska video.
    #[value(name = "webm")]
    WebM,
    /// Require AVI video.
    Avi,
    /// Require a ZIP archive.
    Zip,
    /// Require gzip-compressed data.
    Gzip,
    /// Require a 7-Zip archive.
    #[value(name = "7z")]
    SevenZip,
    /// Require a RAR archive.
    Rar,
    /// Require a tar archive.
    Tar,
    /// Require bzip2-compressed data.
    Bzip2,
    /// Require XZ-compressed data.
    Xz,
    /// Require an ELF executable or object file.
    Elf,
    /// Require a PE (Windows) executable.
    Pe,
    /// Require a Mach-O executable.
    #[value(name = "mach-o")]
    MachO,
    /// Require a WebAssembly module.
    Wasm,
    /// Require a SQLite database.
    Sqlite,
    /// Require a TrueType font.
    Ttf,
    /// Require an OpenType font.
    Otf,
    /// Require a WOFF 1.0 web font.
    Woff,
    /// Require a WOFF 2.0 web font.
    Woff2,
    /// Require a JSON document.
    Json,
    /// Require an XML document.
    Xml,
    /// Require an HTML document.
    Html,
    /// Require plain text.
    #[value(name = "plain-text")]
    PlainText,
    /// Require a PEM-encoded certificate or key.
    Pem,
}

impl From<ExpectedFileType> for FileTypeKind {
    fn from(expected: ExpectedFileType) -> Self {
        match expected {
            ExpectedFileType::Pdf => FileTypeKind::Pdf,
            ExpectedFileType::Rtf => FileTypeKind::Rtf,
            ExpectedFileType::Docx => FileTypeKind::Docx,
            ExpectedFileType::Xlsx => FileTypeKind::Xlsx,
            ExpectedFileType::Pptx => FileTypeKind::Pptx,
            ExpectedFileType::Odt => FileTypeKind::Odt,
            ExpectedFileType::Ods => FileTypeKind::Ods,
            ExpectedFileType::Odp => FileTypeKind::Odp,
            ExpectedFileType::Epub => FileTypeKind::Epub,
            ExpectedFileType::Jar => FileTypeKind::Jar,
            ExpectedFileType::Png => FileTypeKind::Png,
            ExpectedFileType::Jpeg => FileTypeKind::Jpeg,
            ExpectedFileType::Gif => FileTypeKind::Gif,
            ExpectedFileType::Bmp => FileTypeKind::Bmp,
            ExpectedFileType::WebP => FileTypeKind::WebP,
            ExpectedFileType::Tiff => FileTypeKind::Tiff,
            ExpectedFileType::Ico => FileTypeKind::Ico,
            ExpectedFileType::Svg => FileTypeKind::Svg,
            ExpectedFileType::Psd => FileTypeKind::Psd,
            ExpectedFileType::Mp3 => FileTypeKind::Mp3,
            ExpectedFileType::Wav => FileTypeKind::Wav,
            ExpectedFileType::Flac => FileTypeKind::Flac,
            ExpectedFileType::Ogg => FileTypeKind::Ogg,
            ExpectedFileType::Aac => FileTypeKind::Aac,
            ExpectedFileType::M4a => FileTypeKind::M4a,
            ExpectedFileType::Midi => FileTypeKind::Midi,
            ExpectedFileType::Mp4 => FileTypeKind::Mp4,
            ExpectedFileType::WebM => FileTypeKind::WebM,
            ExpectedFileType::Avi => FileTypeKind::Avi,
            ExpectedFileType::Zip => FileTypeKind::Zip,
            ExpectedFileType::Gzip => FileTypeKind::Gzip,
            ExpectedFileType::SevenZip => FileTypeKind::SevenZip,
            ExpectedFileType::Rar => FileTypeKind::Rar,
            ExpectedFileType::Tar => FileTypeKind::Tar,
            ExpectedFileType::Bzip2 => FileTypeKind::Bzip2,
            ExpectedFileType::Xz => FileTypeKind::Xz,
            ExpectedFileType::Elf => FileTypeKind::Elf,
            ExpectedFileType::Pe => FileTypeKind::Pe,
            ExpectedFileType::MachO => FileTypeKind::MachO,
            ExpectedFileType::Wasm => FileTypeKind::Wasm,
            ExpectedFileType::Sqlite => FileTypeKind::Sqlite,
            ExpectedFileType::Ttf => FileTypeKind::Ttf,
            ExpectedFileType::Otf => FileTypeKind::Otf,
            ExpectedFileType::Woff => FileTypeKind::Woff,
            ExpectedFileType::Woff2 => FileTypeKind::Woff2,
            ExpectedFileType::Json => FileTypeKind::Json,
            ExpectedFileType::Xml => FileTypeKind::Xml,
            ExpectedFileType::Html => FileTypeKind::Html,
            ExpectedFileType::PlainText => FileTypeKind::PlainText,
            ExpectedFileType::Pem => FileTypeKind::Pem,
        }
    }
}

pub fn exec(args: FileArgs) -> Result<(), ValidError> {
    let data = read_input(&args.input)?;
    let options = FileTypeOptions {
        data,
        file_name: file_name(&args.input),
        expected: args.expected.map(Into::into),
    };
    let report = detect_file_type(&options);
    if !args.no_extension_check {
        if let Some(false) = report.extension_matches {
            let message = extension_mismatch_message(&report);
            if args.strict {
                return Err(ValidError::File(message));
            }
            eprintln!("warning: {message}");
        }
    }
    if args.verbose {
        print_report(&report);
    }
    match report.detected {
        FileTypeKind::Unknown => {
            let message = match options.expected {
                Some(expected) => format!("expected {} but detected unknown", expected.name()),
                None => "unknown file type".to_string(),
            };
            Err(ValidError::File(message))
        }
        kind => {
            if let Some(expected) = options.expected {
                if expected != kind {
                    return Err(ValidError::File(format!("expected {} but detected {}", expected.name(), kind.name())));
                }
            }
            if !args.verbose {
                println!("{}", kind.name());
            }
            Ok(())
        }
    }
}

fn read_input(input: &str) -> Result<Vec<u8>, GenericError> {
    if input == "-" {
        let mut data = Vec::new();
        std::io::stdin()
            .read_to_end(&mut data)
            .map_err(|error| GenericError::Io(format!("cannot read data from stdin: {error}")))?;
        return Ok(data);
    }
    fs::read(input).map_err(|error| GenericError::Io(format!("cannot read input file \"{input}\": {error}")))
}

fn file_name(input: &str) -> Option<String> {
    if input == "-" {
        return None;
    }
    Path::new(input)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
}

fn extension_mismatch_message(report: &FileTypeReport) -> String {
    match &report.extension {
        Some(extension) => {
            format!("detected {} but the extension \"{extension}\" does not match", report.detected.name())
        }
        None => format!("detected {} but the file extension does not match", report.detected.name()),
    }
}

// -------- Report output --------

fn print_report(report: &FileTypeReport) {
    println!("type:       {}", report.detected.name());
    println!("mime:       {}", report.mime_type.unwrap_or("unknown"));
    println!("confidence: {}", confidence_name(report.confidence));
    if let Some(extension) = &report.extension {
        println!("extension:  {extension}");
        let matches = match report.extension_matches {
            Some(true) => "yes",
            Some(false) => "no",
            None => "unknown",
        };
        println!("extension matches: {matches}");
    }
    if let Some(matches) = report.expected_matches {
        println!("expected:   {}", if matches { "yes" } else { "no" });
    }
    if !report.layers.is_empty() {
        println!("layers:");
        for layer in &report.layers {
            println!("  - {layer}");
        }
    }
}

fn confidence_name(confidence: FileConfidence) -> &'static str {
    match confidence {
        FileConfidence::High => "high",
        FileConfidence::Medium => "medium",
        FileConfidence::Low => "low",
        FileConfidence::None => "none",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1A\npadding";

    fn args(input: &str) -> FileArgs {
        FileArgs {
            input: input.to_string(),
            expected: None,
            no_extension_check: false,
            strict: false,
            verbose: false,
        }
    }

    fn write_temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("oxide-file-test-{nonce}-{name}"));
        std::fs::write(&path, bytes).expect("write fixture");
        path
    }

    // -------- exec --------

    #[test]
    fn exec_detects_a_known_file() {
        let path = write_temp("fixture.png", PNG_BYTES);
        let result = exec(args(&path.to_string_lossy()));
        std::fs::remove_file(&path).ok();
        assert!(result.is_ok());
    }

    #[test]
    fn exec_unknown_binary_errors() {
        let path = write_temp("fixture.bin", &[0x00, 0x01, 0x02, 0x03, 0x04]);
        let result = exec(args(&path.to_string_lossy()));
        std::fs::remove_file(&path).ok();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown file type"));
    }

    #[test]
    fn exec_missing_file_errors() {
        let result = exec(args("/nonexistent/definitely-missing.png"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot read input file"));
    }

    #[test]
    fn exec_expected_match_and_mismatch() {
        let path = write_temp("fixture.png", PNG_BYTES);
        let input = path.to_string_lossy().to_string();
        let matched = exec(FileArgs {
            expected: Some(ExpectedFileType::Png),
            ..args(&input)
        });
        assert!(matched.is_ok());

        let mismatched = exec(FileArgs {
            expected: Some(ExpectedFileType::Jpeg),
            ..args(&input)
        });
        std::fs::remove_file(&path).ok();
        assert!(mismatched.is_err());
        assert!(
            mismatched
                .unwrap_err()
                .to_string()
                .contains("expected jpeg but detected png")
        );
    }

    #[test]
    fn exec_expected_unknown_message() {
        let path = write_temp("fixture.bin", &[0x00, 0x01, 0x02]);
        let input = path.to_string_lossy().to_string();
        let result = exec(FileArgs {
            expected: Some(ExpectedFileType::Png),
            ..args(&input)
        });
        std::fs::remove_file(&path).ok();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected png but detected unknown")
        );
    }

    #[test]
    fn exec_extension_mismatch_warns_or_errors() {
        let path = write_temp("fixture.jpg", PNG_BYTES);
        let input = path.to_string_lossy().to_string();

        // Without --strict the mismatch is a warning and the command succeeds.
        let warned = exec(args(&input));
        assert!(warned.is_ok());

        let strict = exec(FileArgs {
            strict: true,
            ..args(&input)
        });
        std::fs::remove_file(&path).ok();
        assert!(strict.is_err());
        assert!(
            strict
                .unwrap_err()
                .to_string()
                .contains("detected png but the extension \"jpg\" does not match")
        );
    }

    #[test]
    fn exec_verbose_known_and_unknown() {
        let path = write_temp("fixture.png", PNG_BYTES);
        let known = exec(FileArgs {
            verbose: true,
            ..args(&path.to_string_lossy())
        });
        assert!(known.is_ok());

        let unknown_path = write_temp("fixture.bin", &[0x00, 0x01, 0x02]);
        let unknown = exec(FileArgs {
            verbose: true,
            ..args(&unknown_path.to_string_lossy())
        });
        std::fs::remove_file(&path).ok();
        std::fs::remove_file(&unknown_path).ok();
        assert!(unknown.is_err());
    }

    // -------- Conversion --------

    #[test]
    fn expected_file_type_maps_to_kind() {
        assert_eq!(FileTypeKind::from(ExpectedFileType::Pdf), FileTypeKind::Pdf);
        assert_eq!(FileTypeKind::from(ExpectedFileType::SevenZip), FileTypeKind::SevenZip);
        assert_eq!(FileTypeKind::from(ExpectedFileType::WebM), FileTypeKind::WebM);
        assert_eq!(FileTypeKind::from(ExpectedFileType::PlainText), FileTypeKind::PlainText);
    }
}
