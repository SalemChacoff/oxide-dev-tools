mod codecs;
mod generators;

pub use codecs::base64_codec::{Base64Alphabet, Base64Error, Base64Kind, Base64Options, convert_base64};
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
