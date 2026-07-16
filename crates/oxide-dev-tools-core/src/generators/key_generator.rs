use rand::seq::SliceRandom;

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

#[derive(Debug)]
pub enum KeyKind {
    Password(PasswordOptions),
    Token,
}

pub fn generate_key(kind: KeyKind) -> String {
    match kind {
        KeyKind::Password(opts) => gen_password(&opts),
        KeyKind::Token => gen_token(),
    }
}

fn gen_password(opts: &PasswordOptions) -> String {
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
        panic!("at least one character set must be selected");
    }

    let mut rng = rand::thread_rng();
    let mut password = Vec::with_capacity(opts.length);

    // Guarantee at least one character from each selected set
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

    // Fill remaining slots from the combined allowed set
    for _ in password.len()..opts.length {
        password.push(*allowed.choose(&mut rng).unwrap());
    }

    // Shuffle so guaranteed characters aren't in predictable positions
    password.shuffle(&mut rng);

    String::from_utf8(password).expect("generated password is not valid UTF-8")
}

fn gen_token() -> String {
    "Token".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_default_length() {
        let opts = PasswordOptions::default();
        let pwd = gen_password(&opts);
        assert_eq!(pwd.len(), 16);
    }

    #[test]
    fn password_custom_length() {
        let opts = PasswordOptions {
            length: 32,
            ..Default::default()
        };
        let pwd = gen_password(&opts);
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
        let pwd = gen_password(&opts);
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
        let pwd = gen_password(&opts);
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
        let pwd = gen_password(&opts);
        assert!(pwd.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn password_includes_each_selected_set() {
        let opts = PasswordOptions::default();
        let pwd = gen_password(&opts);
        assert!(pwd.chars().any(|c| c.is_ascii_lowercase()), "missing lowercase");
        assert!(pwd.chars().any(|c| c.is_ascii_uppercase()), "missing uppercase");
        assert!(pwd.chars().any(|c| c.is_ascii_digit()), "missing digit");
    }

    #[test]
    #[should_panic(expected = "at least one character set must be selected")]
    fn password_no_character_set_panics() {
        let opts = PasswordOptions {
            lowercase: false,
            uppercase: false,
            digits: false,
            special: false,
            length: 16,
        };
        gen_password(&opts);
    }

    #[test]
    fn password_different_each_call() {
        let opts = PasswordOptions::default();
        let a = gen_password(&opts);
        let b = gen_password(&opts);
        // Highly improbable to collide on a 16-char random string
        assert_ne!(a, b);
    }

    #[test]
    fn generate_key_password_dispatch() {
        let result = generate_key(KeyKind::Password(PasswordOptions::default()));
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn password_special_chars_included() {
        let special_set: &[u8] = b"!@#$%^&*()-_=+[]{}|;:,.<>?/";
        let opts = PasswordOptions {
            lowercase: false,
            uppercase: false,
            digits: false,
            special: true,
            length: 64,
        };
        let pwd = gen_password(&opts);
        assert!(pwd.bytes().any(|b| special_set.contains(&b)));
    }
}
