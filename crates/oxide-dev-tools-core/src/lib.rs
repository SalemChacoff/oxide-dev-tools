mod codecs;
mod converters;
mod generators;
mod validators;

pub use codecs::base64_codec::{Base64Alphabet, Base64Error, Base64Kind, Base64Options, convert_base64};
pub use codecs::hex_codec::{HexCase, HexError, HexKind, HexOptions, convert_hex};
pub use codecs::url_codec::{UrlError, UrlKind, UrlMode, UrlOptions, convert_url};
pub use converters::doc_converter::{DocError, DocKind, DocOptions, convert_doc};
pub use converters::timestamp_converter::{
    TimestampError, TimestampInputFormat, TimestampKind, TimestampOptions, TimestampOutputFormat, TimestampUnit,
    TimestampZone, convert_timestamp, parse_timestamp_zone,
};
pub use converters::unit_converter::{UnitCategory, UnitError, UnitKind, UnitOptions, convert_unit, unit_catalog};
pub use generators::fake_generator::{FakeError, FakeKind, FakeOptions, generate_fake};
pub use generators::id_generator::{IdError, IdKind, generate_id};
pub use generators::jwt_generator::{JwtError, JwtOptions, generate_jwt};
pub use generators::key_generator::{KeyError, KeyKind, PasswordOptions, TokenEncoding, TokenOptions, generate_key};
pub use generators::lorem_generator::{
    LoremError, LoremKind, ParagraphOptions, SentenceOptions, WordOptions, generate_lorem,
};
pub use generators::sample_file_generator::{
    JpgOptions, PdfOptions, PngOptions, SampleError, SampleKind, TamperKind, generate_sample_file,
};
pub use validators::card_validator::{CardNetwork, CardOptions, CardReport, validate_card};
pub use validators::email_validator::{EmailDomainKind, EmailMode, EmailOptions, EmailReport, validate_email};
pub use validators::file_type_validator::{
    FileConfidence, FileTypeKind, FileTypeOptions, FileTypeReport, detect_file_type,
};
pub use validators::ip_validator::{IpClass, IpMode, IpOptions, IpReport, IpVersion, validate_ip};
pub use validators::password_validator::{
    CrackTimeReport, PasswordReport, PasswordStrength, PasswordStrengthOptions, validate_password,
};
pub use validators::syntax_validator::{SyntaxFormat, SyntaxKind, SyntaxOptions, SyntaxReport, validate_syntax};
pub use validators::url_validator::{UrlHostKind, UrlReport, UrlValidationOptions, validate_url};
pub use validators::uuid_validator::{UuidKind, UuidOptions, UuidReport, UuidVariant, UuidVersion, validate_uuid};
