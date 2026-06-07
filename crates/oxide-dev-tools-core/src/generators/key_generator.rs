#[derive(Debug)]
pub enum KeyKind {
    Password,
    Token,
}

pub fn generate_key(kind: KeyKind) -> String {
    match kind {
        KeyKind::Password => gen_password(),
        KeyKind::Token => gen_token(),
    }
}

fn gen_password() -> String {
    "Password".to_string()
}

fn gen_token() -> String {
    "Token".to_string()
}
