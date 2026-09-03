use std::io::Write;

use clap::{Args, Subcommand};
use oxide_dev_tools_core::*;

use crate::error::{GenError, GenericError};

/// `oxide gen sample [subcommand]` — sample file generator dispatch
#[derive(Args)]
pub struct SampleArgs {
    #[command(subcommand)]
    pub kind: SampleCmd,
}

#[derive(Subcommand)]
pub enum SampleCmd {
    /// Generate a PDF document
    #[command(name = "pdf")]
    Pdf {
        /// Exact byte size, e.g. 5120, 5kb, 5MB
        #[arg(long)]
        size: Option<String>,
        /// Number of pages (1-1000)
        #[arg(long, default_value_t = 1)]
        pages: u32,
        /// Text drawn on every page
        #[arg(long)]
        text: Option<String>,
        /// Output path; "-" writes to stdout. Defaults to sample.pdf
        #[arg(long)]
        output: Option<String>,
        /// Corrupt the file after generation to test server validation
        #[arg(long, value_enum)]
        tamper: Option<TamperCli>,
        /// Write the file with a different extension (e.g. txt)
        #[arg(long)]
        wrong_ext: Option<String>,
    },
    /// Generate a PNG image
    #[command(name = "png")]
    Png {
        /// Exact byte size, e.g. 5120, 5kb, 5MB
        #[arg(long)]
        size: Option<String>,
        /// Image width in pixels (defaults to a size that fits)
        #[arg(long)]
        width: Option<u32>,
        /// Image height in pixels (defaults to a size that fits)
        #[arg(long)]
        height: Option<u32>,
        /// Pixel color: RRGGBB hex (e.g. ff0000) or red/green/blue/black/white/gray
        #[arg(long)]
        color: Option<String>,
        /// Output path; "-" writes to stdout. Defaults to sample.png
        #[arg(long)]
        output: Option<String>,
        /// Corrupt the file after generation to test server validation
        #[arg(long, value_enum)]
        tamper: Option<TamperCli>,
        /// Write the file with a different extension (e.g. txt)
        #[arg(long)]
        wrong_ext: Option<String>,
    },
    /// Generate a JPEG image (1x1 baseline, padded with comments)
    #[command(name = "jpg")]
    Jpg {
        /// Exact byte size, e.g. 5120, 5kb, 5MB
        #[arg(long)]
        size: Option<String>,
        /// Output path; "-" writes to stdout. Defaults to sample.jpg
        #[arg(long)]
        output: Option<String>,
        /// Corrupt the file after generation to test server validation
        #[arg(long, value_enum)]
        tamper: Option<TamperCli>,
        /// Write the file with a different extension (e.g. txt)
        #[arg(long)]
        wrong_ext: Option<String>,
    },
}

/// CLI-side tamper selectors, mapped 1:1 onto [`TamperKind`].
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum TamperCli {
    /// Zero out the leading magic bytes
    Magic,
    /// Keep only the first half of the file
    Truncate,
    /// Replace every byte with zero
    Zeros,
    /// Replace every byte with the letter "a"
    Text,
}

impl From<TamperCli> for TamperKind {
    fn from(value: TamperCli) -> Self {
        match value {
            TamperCli::Magic => TamperKind::MagicBytes,
            TamperCli::Truncate => TamperKind::Truncate,
            TamperCli::Zeros => TamperKind::ZeroFill,
            TamperCli::Text => TamperKind::TextFill,
        }
    }
}

pub fn exec(args: SampleArgs) -> Result<(), GenError> {
    match args.kind {
        SampleCmd::Pdf {
            size,
            pages,
            text,
            output,
            tamper,
            wrong_ext,
        } => {
            let kind = SampleKind::Pdf(PdfOptions {
                size: parse_size(size)?,
                pages,
                text,
            });
            write_output(&generate_sample_file(kind, tamper.map(Into::into))?, output, wrong_ext.as_deref(), "pdf")?;
        }
        SampleCmd::Png {
            size,
            width,
            height,
            color,
            output,
            tamper,
            wrong_ext,
        } => {
            let kind = SampleKind::Png(PngOptions {
                size: parse_size(size)?,
                width,
                height,
                color,
            });
            write_output(&generate_sample_file(kind, tamper.map(Into::into))?, output, wrong_ext.as_deref(), "png")?;
        }
        SampleCmd::Jpg {
            size,
            output,
            tamper,
            wrong_ext,
        } => {
            let kind = SampleKind::Jpg(JpgOptions {
                size: parse_size(size)?,
            });
            write_output(&generate_sample_file(kind, tamper.map(Into::into))?, output, wrong_ext.as_deref(), "jpg")?;
        }
    }
    Ok(())
}

/// Parse a human size string (`5120`, `5kb`, `5MB`, `1gb`, SI base 1000)
/// into an exact byte count.
fn parse_size(size: Option<String>) -> Result<Option<u64>, GenericError> {
    let Some(raw) = size else {
        return Ok(None);
    };
    let value = raw.trim().to_ascii_lowercase();
    let (digits, multiplier) = match value.as_str() {
        suffix if suffix.ends_with("gb") => (&suffix[..suffix.len() - 2], 1_000_000_000u64),
        suffix if suffix.ends_with("mb") => (&suffix[..suffix.len() - 2], 1_000_000u64),
        suffix if suffix.ends_with("kb") => (&suffix[..suffix.len() - 2], 1_000u64),
        suffix if suffix.ends_with('b') => (&suffix[..suffix.len() - 1], 1u64),
        digits => (digits, 1u64),
    };
    let number: u64 = digits
        .trim()
        .parse()
        .map_err(|_| GenericError::from(format!("invalid size \"{raw}\", expected e.g. 5120, 5kb, 5MB")))?;
    Ok(Some(
        number
            .checked_mul(multiplier)
            .ok_or_else(|| GenericError::from(format!("size \"{raw}\" is too large")))?,
    ))
}

/// Pick the destination path: `--output` wins, `--wrong-ext` overrides the
/// extension, and an omitted output defaults to `sample.<ext>` in the cwd.
fn resolve_output(output: Option<String>, wrong_ext: Option<&str>, extension: &str) -> String {
    let Some(user_path) = output else {
        let ext = wrong_ext.unwrap_or(extension).trim_start_matches('.');
        return format!("sample.{ext}");
    };
    if user_path == "-" {
        return user_path;
    }
    let Some(ext) = wrong_ext else {
        return user_path;
    };
    let mut path = std::path::PathBuf::from(user_path);
    path.set_extension(ext.trim_start_matches('.'));
    path.to_string_lossy().into_owned()
}

/// Write `bytes` to the resolved output path (or stdout when it is "-").
fn write_output(
    bytes: &[u8],
    output: Option<String>,
    wrong_ext: Option<&str>,
    extension: &str,
) -> Result<(), GenericError> {
    let path = resolve_output(output, wrong_ext, extension);
    if path == "-" {
        std::io::stdout()
            .write_all(bytes)
            .map_err(|err| GenericError::Io(format!("failed to write to stdout: {err}")))?;
        return Ok(());
    }
    std::fs::write(&path, bytes).map_err(|err| GenericError::Io(format!("failed to write \"{path}\": {err}")))?;
    println!("{path}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn temp_path(extension: &str) -> String {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("oxide-sample-test-{nonce}.{extension}"))
            .to_string_lossy()
            .into_owned()
    }

    // -------- parse_size --------

    #[test]
    fn parse_size_none_returns_none() {
        assert!(parse_size(None).unwrap().is_none());
    }

    #[test]
    fn parse_size_plain_bytes() {
        assert_eq!(parse_size(Some("5120".into())).unwrap(), Some(5120));
    }

    #[test]
    fn parse_size_si_units() {
        assert_eq!(parse_size(Some("5kb".into())).unwrap(), Some(5_000));
        assert_eq!(parse_size(Some("5KB".into())).unwrap(), Some(5_000));
        assert_eq!(parse_size(Some("5mb".into())).unwrap(), Some(5_000_000));
        assert_eq!(parse_size(Some("1gb".into())).unwrap(), Some(1_000_000_000));
        assert_eq!(parse_size(Some("5b".into())).unwrap(), Some(5));
    }

    #[test]
    fn parse_size_trimmed_value() {
        assert_eq!(parse_size(Some(" 5 kb ".into())).unwrap(), Some(5_000));
    }

    #[test]
    fn parse_size_zero_is_valid() {
        assert_eq!(parse_size(Some("0".into())).unwrap(), Some(0));
    }

    #[test]
    fn parse_size_garbage_errors() {
        let err = parse_size(Some("abc".into())).unwrap_err().to_string();
        assert!(err.contains("invalid size"));
    }

    #[test]
    fn parse_size_unknown_suffix_errors() {
        let err = parse_size(Some("5xb".into())).unwrap_err().to_string();
        assert!(err.contains("invalid size"));
    }

    #[test]
    fn parse_size_overflow_errors() {
        let err = parse_size(Some("18446744073709551615gb".into()))
            .unwrap_err()
            .to_string();
        assert!(err.contains("too large"));
    }

    // -------- resolve_output --------

    #[test]
    fn resolve_output_defaults_to_sample_extension() {
        let resolved = resolve_output(None, None, "pdf");
        let path = Path::new(&resolved);
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "sample.pdf");
    }

    #[test]
    fn resolve_output_default_with_wrong_ext() {
        let resolved = resolve_output(None, Some("txt"), "pdf");
        let path = Path::new(&resolved);
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "sample.txt");
    }

    #[test]
    fn resolve_output_explicit_path_kept() {
        let resolved = resolve_output(Some("custom.pdf".into()), None, "pdf");
        let path = Path::new(&resolved);
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "custom.pdf");
    }

    #[test]
    fn resolve_output_explicit_path_with_wrong_ext() {
        let resolved = resolve_output(Some("custom.pdf".into()), Some("txt"), "pdf");
        let path = Path::new(&resolved);
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "custom.txt");
    }

    #[test]
    fn resolve_output_wrong_ext_with_parent_dir() {
        let path = resolve_output(Some("dir/custom.pdf".into()), Some("jpg"), "pdf");
        assert!(path.contains("dir"), "parent kept: {path}");
        assert!(path.ends_with("custom.jpg"), "extension replaced: {path}");
    }

    #[test]
    fn resolve_output_stdout_marker() {
        assert_eq!(resolve_output(Some("-".into()), None, "pdf"), "-");
    }

    #[test]
    fn resolve_output_strips_leading_dot_in_ext() {
        let resolved = resolve_output(Some("custom.pdf".into()), Some(".txt"), "pdf");
        let path = Path::new(&resolved);
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "custom.txt");
    }

    // -------- exec --------

    #[test]
    fn exec_pdf_writes_exact_size_file() {
        let path = temp_path("pdf");
        let result = exec(SampleArgs {
            kind: SampleCmd::Pdf {
                size: Some("5kb".into()),
                pages: 1,
                text: None,
                output: Some(path.clone()),
                tamper: None,
                wrong_ext: None,
            },
        });
        assert!(result.is_ok());
        let bytes = std::fs::read(&path).expect("output file missing");
        assert_eq!(bytes.len(), 5_000);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn exec_png_writes_exact_size_file() {
        let path = temp_path("png");
        let result = exec(SampleArgs {
            kind: SampleCmd::Png {
                size: Some("5mb".into()),
                width: None,
                height: None,
                color: Some("red".into()),
                output: Some(path.clone()),
                tamper: None,
                wrong_ext: None,
            },
        });
        assert!(result.is_ok());
        let bytes = std::fs::read(&path).expect("output file missing");
        assert_eq!(bytes.len(), 5_000_000);
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn exec_jpg_writes_magic_file() {
        let path = temp_path("jpg");
        let result = exec(SampleArgs {
            kind: SampleCmd::Jpg {
                size: Some("2kb".into()),
                output: Some(path.clone()),
                tamper: None,
                wrong_ext: None,
            },
        });
        assert!(result.is_ok());
        let bytes = std::fs::read(&path).expect("output file missing");
        assert_eq!(bytes.len(), 2_000);
        assert_eq!(&bytes[..2], &[0xFF, 0xD8]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn exec_wrong_ext_changes_extension_only() {
        let base = temp_path("pdf");
        let mut target = std::path::PathBuf::from(&base);
        target.set_extension("txt");
        let result = exec(SampleArgs {
            kind: SampleCmd::Pdf {
                size: Some("2kb".into()),
                pages: 1,
                text: None,
                output: Some(base),
                tamper: None,
                wrong_ext: Some("txt".into()),
            },
        });
        assert!(result.is_ok());
        let bytes = std::fs::read(&target).expect("output file missing");
        assert!(bytes.starts_with(b"%PDF-1.4"), "bytes stay pdf despite .txt name");
        std::fs::remove_file(&target).ok();
    }

    #[test]
    fn exec_tamper_magic_zeroes_signature() {
        let path = temp_path("pdf");
        let result = exec(SampleArgs {
            kind: SampleCmd::Pdf {
                size: Some("2kb".into()),
                pages: 1,
                text: None,
                output: Some(path.clone()),
                tamper: Some(TamperCli::Magic),
                wrong_ext: None,
            },
        });
        assert!(result.is_ok());
        let bytes = std::fs::read(&path).expect("output file missing");
        assert_eq!(bytes.len(), 2_000);
        assert_eq!(&bytes[..4], &[0, 0, 0, 0]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn exec_size_below_minimum_errors() {
        let result = exec(SampleArgs {
            kind: SampleCmd::Pdf {
                size: Some("1".into()),
                pages: 1,
                text: None,
                output: None,
                tamper: None,
                wrong_ext: None,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("below the minimum"));
    }

    #[test]
    fn exec_invalid_size_errors() {
        let result = exec(SampleArgs {
            kind: SampleCmd::Png {
                size: Some("nope".into()),
                width: None,
                height: None,
                color: None,
                output: None,
                tamper: None,
                wrong_ext: None,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid size"));
    }

    #[test]
    fn exec_invalid_color_errors() {
        let result = exec(SampleArgs {
            kind: SampleCmd::Png {
                size: None,
                width: None,
                height: None,
                color: Some("chartreuse".into()),
                output: None,
                tamper: None,
                wrong_ext: None,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("color"));
    }

    #[test]
    fn exec_pages_beyond_limit_errors() {
        let result = exec(SampleArgs {
            kind: SampleCmd::Pdf {
                size: None,
                pages: 1001,
                text: None,
                output: None,
                tamper: None,
                wrong_ext: None,
            },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pages"));
    }
}
