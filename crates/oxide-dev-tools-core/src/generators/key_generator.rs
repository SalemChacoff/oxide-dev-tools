use std::fmt;

use rand::seq::{IndexedRandom, SliceRandom};

/// Errors that can occur when generating a key/token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// No character set was selected for password generation.
    NoCharacterSet,
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyError::NoCharacterSet => write!(f, "at least one character set must be selected"),
        }
    }
}

impl std::error::Error for KeyError {}

/// Kinds of keys/tokens that can be generated.
#[derive(Debug)]
pub enum KeyKind {
    Password(PasswordOptions),
    Token(TokenOptions),
}

/// Generate a key or token according to `kind`.
pub fn generate_key(kind: KeyKind) -> Result<String, KeyError> {
    match kind {
        KeyKind::Password(opts) => gen_password(&opts),
        KeyKind::Token(opts) => gen_token(&opts),
    }
}

// -------- Password generator --------

/// Options for password generation.
#[derive(Debug, Clone)]
pub struct PasswordOptions {
    pub length: usize,
    pub lowercase: bool,
    pub uppercase: bool,
    pub digits: bool,
    pub special: bool,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 16,
            lowercase: true,
            uppercase: true,
            digits: true,
            special: false,
        }
    }
}

fn gen_password(opts: &PasswordOptions) -> Result<String, KeyError> {
    let lowercase = b"abcdefghijklmnopqrstuvwxyz";
    let uppercase = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let digits = b"0123456789";
    let special = b"!@#$%^&*";

    let mut allowed = Vec::new();

    if opts.lowercase {
        allowed.extend_from_slice(lowercase);
    }
    if opts.uppercase {
        allowed.extend_from_slice(uppercase);
    }
    if opts.digits {
        allowed.extend_from_slice(digits);
    }
    if opts.special {
        allowed.extend_from_slice(special);
    }

    if allowed.is_empty() {
        return Err(KeyError::NoCharacterSet);
    }

    let mut rng = rand::rng();
    let mut password = Vec::with_capacity(opts.length);

    // Guarantee at least one character from each selected set.
    if opts.lowercase {
        password.push(*lowercase.choose(&mut rng).unwrap());
    }
    if opts.uppercase {
        password.push(*uppercase.choose(&mut rng).unwrap());
    }
    if opts.digits {
        password.push(*digits.choose(&mut rng).unwrap());
    }
    if opts.special {
        password.push(*special.choose(&mut rng).unwrap());
    }

    // Fill remaining slots from the combined allowed set.
    for _ in password.len()..opts.length {
        password.push(*allowed.choose(&mut rng).unwrap());
    }

    // Shuffle so guaranteed characters aren't in predictable positions.
    password.shuffle(&mut rng);

    Ok(String::from_utf8(password).expect("generated password is not valid UTF-8"))
}

// -------- Token generator --------

/// Encoding for generated tokens.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenEncoding {
    /// Lowercase hexadecimal (output length = 2 × byte length).
    #[default]
    Hex,
    /// Standard base64 without padding.
    Base64,
}

/// Options for token generation.
#[derive(Debug, Clone)]
pub struct TokenOptions {
    pub length: usize,
    pub encoding: TokenEncoding,
}

impl Default for TokenOptions {
    fn default() -> Self {
        Self {
            length: 32,
            encoding: TokenEncoding::Hex,
        }
    }
}

fn gen_token(opts: &TokenOptions) -> Result<String, KeyError> {
    let bytes: Vec<u8> = (0..opts.length).map(|_| rand::random::<u8>()).collect();

    match opts.encoding {
        TokenEncoding::Hex => Ok(bytes_to_hex(&bytes)),
        TokenEncoding::Base64 => Ok(base64_encode_no_pad(&bytes)),
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

fn base64_encode_no_pad(bytes: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_default_length() {
        let opts = PasswordOptions::default();
        let pwd = gen_password(&opts).unwrap();
        assert_eq!(pwd.len(), 16);
    }

    #[test]
    fn password_custom_length() {
        let opts = PasswordOptions {
            length: 32,
            ..Default::default()
        };
        let pwd = gen_password(&opts).unwrap();
        assert_eq!(pwd.len(), 32);
    }

    #[test]
    fn password_contains_lowercase() {
        let opts = PasswordOptions {
            uppercase: false,
            digits: false,
            special: false,
            ..Default::default()
        };
        let pwd = gen_password(&opts).unwrap();
        assert!(pwd.chars().all(|c| c.is_ascii_lowercase()));
    }

    #[test]
    fn password_contains_uppercase() {
        let opts = PasswordOptions {
            lowercase: false,
            digits: false,
            special: false,
            ..Default::default()
        };
        let pwd = gen_password(&opts).unwrap();
        assert!(pwd.chars().all(|c| c.is_ascii_uppercase()));
    }

    #[test]
    fn password_contains_digits() {
        let opts = PasswordOptions {
            lowercase: false,
            uppercase: false,
            special: false,
            ..Default::default()
        };
        let pwd = gen_password(&opts).unwrap();
        assert!(pwd.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn password_includes_each_selected_set() {
        let opts = PasswordOptions::default();
        let pwd = gen_password(&opts).unwrap();
        assert!(pwd.chars().any(|c| c.is_ascii_lowercase()), "missing lowercase");
        assert!(pwd.chars().any(|c| c.is_ascii_uppercase()), "missing uppercase");
        assert!(pwd.chars().any(|c| c.is_ascii_digit()), "missing digit");
    }

    #[test]
    fn password_no_character_set_errors() {
        let opts = PasswordOptions {
            lowercase: false,
            uppercase: false,
            digits: false,
            special: false,
            length: 16,
        };
        assert_eq!(gen_password(&opts), Err(KeyError::NoCharacterSet));
    }

    #[test]
    fn password_different_each_call() {
        let opts = PasswordOptions::default();
        let a = gen_password(&opts).unwrap();
        let b = gen_password(&opts).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn generate_key_password_dispatch() {
        let result = generate_key(KeyKind::Password(PasswordOptions::default()));
        assert_eq!(result.unwrap().len(), 16);
    }

    #[test]
    fn password_special_chars_included() {
        let special_set: &[u8] = b"!@#$%^&*";
        let opts = PasswordOptions {
            lowercase: false,
            uppercase: false,
            digits: false,
            special: true,
            length: 64,
        };
        let pwd = gen_password(&opts).unwrap();
        assert!(pwd.bytes().any(|b| special_set.contains(&b)));
    }

    #[test]
    fn token_hex_default_length() {
        let opts = TokenOptions::default();
        let token = gen_token(&opts).unwrap();
        assert_eq!(token.len(), 64); // 32 bytes × 2 hex chars
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn token_base64_length() {
        let opts = TokenOptions {
            length: 30,
            encoding: TokenEncoding::Base64,
        };
        let token = gen_token(&opts).unwrap();
        assert!(!token.is_empty());
        assert!(!token.contains('='));
    }

    #[test]
    fn token_different_each_call() {
        let opts = TokenOptions::default();
        let a = gen_token(&opts).unwrap();
        let b = gen_token(&opts).unwrap();
        assert_ne!(a, b);
    }
}
