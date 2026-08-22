//! JWT (HS256) generation from a JSON object payload.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use std::fmt;

/// Errors that can occur when generating a JWT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtError {
    /// The payload is not valid JSON.
    InvalidJson(String),
    /// The payload is a valid JSON value but not an object.
    NotAnObject,
    /// A required claim is missing from the payload.
    MissingClaim(String),
    /// A claim exists but has the wrong type or value.
    InvalidClaim(String),
    /// The signing secret is empty.
    EmptySecret,
}

impl fmt::Display for JwtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JwtError::InvalidJson(msg) => write!(f, "invalid JSON payload: {msg}"),
            JwtError::NotAnObject => write!(f, "payload must be a JSON object"),
            JwtError::MissingClaim(claim) => {
                write!(f, "payload must contain required claim \"{claim}\"")
            }
            JwtError::InvalidClaim(msg) => write!(f, "invalid claim: {msg}"),
            JwtError::EmptySecret => write!(f, "secret must not be empty"),
        }
    }
}

impl std::error::Error for JwtError {}

/// Options for JWT generation.
#[derive(Debug, Clone)]
pub struct JwtOptions {
    /// Raw JSON object with the token claims. Must contain a non-empty
    /// string `sub` claim; an optional `exp` claim must be a non-negative
    /// integer Unix timestamp in seconds.
    pub payload: String,
    /// HMAC-SHA256 signing secret (HS256); must not be empty.
    pub secret: String,
    /// Expiry as absolute Unix timestamp in seconds. Used only when the
    /// payload does not contain its own `exp` claim (JSON `exp` wins).
    pub exp: Option<u64>,
}

/// Generate an HS256 JWT from `options`.
///
/// The header is fixed to `{"alg":"HS256","typ":"JWT"}` and the payload JSON
/// is normalized to compact form with sorted keys, so identical options
/// always produce the identical token.
pub fn generate_jwt(options: JwtOptions) -> Result<String, JwtError> {
    if options.secret.is_empty() {
        return Err(JwtError::EmptySecret);
    }

    let claims = parse_claims(&options.payload)?;
    let claims = inject_expiry(claims, options.exp);

    let header = serde_json::json!({ "alg": "HS256", "typ": "JWT" });
    let encoded_header = encode_json(&header);
    let encoded_claims = encode_json(&claims);
    let signing_input = format!("{encoded_header}.{encoded_claims}");
    let signature = sign_hmac(signing_input.as_bytes(), options.secret.as_bytes());

    Ok(format!("{signing_input}.{signature}"))
}

// -------- Payload validation --------

fn parse_claims(payload: &str) -> Result<Value, JwtError> {
    let claims = serde_json::from_str::<Value>(payload).map_err(|err| JwtError::InvalidJson(err.to_string()))?;
    if !claims.is_object() {
        return Err(JwtError::NotAnObject);
    }

    match claims.get("sub") {
        Some(Value::String(sub)) if !sub.is_empty() => {}
        Some(_) => {
            return Err(JwtError::InvalidClaim(String::from("claim \"sub\" must be a non-empty string")));
        }
        None => return Err(JwtError::MissingClaim(String::from("sub"))),
    }

    if let Some(exp) = claims.get("exp") {
        if exp.as_u64().is_none() {
            return Err(JwtError::InvalidClaim(String::from(
                "claim \"exp\" must be a non-negative integer Unix timestamp",
            )));
        }
    }

    Ok(claims)
}

fn inject_expiry(mut claims: Value, exp: Option<u64>) -> Value {
    if claims.get("exp").is_some() {
        return claims;
    }
    if let Some(exp) = exp {
        claims["exp"] = Value::from(exp);
    }
    claims
}

// -------- Encoding & signing --------

fn encode_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("parsed JSON value always serializes");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn sign_hmac(input: &[u8], secret: &[u8]) -> String {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret).expect("HMAC accepts keys of any length");
    mac.update(input);
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    const SECRET: &str = "test-secret";

    fn generate(payload: &str, exp: Option<u64>) -> Result<String, JwtError> {
        generate_jwt(JwtOptions {
            payload: String::from(payload),
            secret: String::from(SECRET),
            exp,
        })
    }

    fn decode_segment(segment: &str) -> Value {
        let bytes = URL_SAFE_NO_PAD.decode(segment).expect("segment is valid base64url");
        serde_json::from_slice(&bytes).expect("segment is valid JSON")
    }

    fn verify_signature(token: &str, secret: &str) -> bool {
        let mut parts = token.split('.');
        let signing_input =
            format!("{}.{}", parts.next().expect("header segment"), parts.next().expect("payload segment"));
        let signature = parts.next().expect("signature segment");
        let mut mac =
            <Hmac<Sha256> as Mac>::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
        mac.update(signing_input.as_bytes());
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()) == signature
    }

    #[test]
    fn fixed_header_known_answer() {
        // Base64url of the fixed header `{"alg":"HS256","typ":"JWT"}`.
        let token = generate(r#"{"sub":"user-1"}"#, None).unwrap();
        assert_eq!(token.split('.').next().unwrap(), "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    }

    #[test]
    fn known_answer_vector() {
        // Computed with an independent implementation (Python hmac/hashlib).
        let token = generate_jwt(JwtOptions {
            payload: String::from(r#"{"iat":1516239022,"sub":"oxide-user"}"#),
            secret: String::from("oxide-secret"),
            exp: None,
        })
        .unwrap();
        assert_eq!(
            token,
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE1MTYyMzkwMjIsInN1YiI6Im94aWRlLXVzZXIifQ.jdj3jKqANtF9eN3UFfIHL6niFT8uWLYSxTshxCLb0zs"
        );
    }

    #[test]
    fn valid_token_structure_and_claims() {
        let token = generate(r#"{"sub":"user-1","aud":"app"}"#, None).unwrap();
        let mut parts = token.split('.');
        let header = decode_segment(parts.next().unwrap());
        let claims = decode_segment(parts.next().unwrap());
        let signature = parts.next().unwrap();
        assert!(parts.next().is_none(), "token must have exactly 3 segments");

        assert_eq!(header, serde_json::json!({ "alg": "HS256", "typ": "JWT" }));
        assert_eq!(claims["sub"], "user-1");
        assert_eq!(claims["aud"], "app");
        assert!(claims.get("exp").is_none());
        assert!(!signature.is_empty());
        assert!(verify_signature(&token, SECRET));
    }

    #[test]
    fn deterministic_for_same_input() {
        let payload = r#"{"sub":"user-1","exp":1750000000}"#;
        let a = generate(payload, None).unwrap();
        let b = generate(payload, None).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn missing_sub_errors() {
        let err = generate(r#"{"aud":"app"}"#, None).unwrap_err();
        assert!(err.to_string().contains("\"sub\""));
    }

    #[test]
    fn empty_sub_errors() {
        let err = generate(r#"{"sub":""}"#, None).unwrap_err();
        assert!(err.to_string().contains("\"sub\""));
    }

    #[test]
    fn non_string_sub_errors() {
        let err = generate(r#"{"sub":42}"#, None).unwrap_err();
        assert!(err.to_string().contains("\"sub\""));
    }

    #[test]
    fn non_object_payload_errors() {
        let err = generate("[1, 2, 3]", None).unwrap_err();
        assert!(err.to_string().contains("must be a JSON object"));
    }

    #[test]
    fn invalid_json_errors() {
        let err = generate("not json", None).unwrap_err();
        assert!(err.to_string().contains("invalid JSON payload"));
    }

    #[test]
    fn empty_payload_errors() {
        let err = generate("", None).unwrap_err();
        assert!(err.to_string().contains("invalid JSON payload"));
    }

    #[test]
    fn payload_exp_wins_over_argument() {
        let token = generate(r#"{"sub":"user-1","exp":1700000000}"#, Some(999_999_999)).unwrap();
        let claims = decode_segment(token.split('.').nth(1).unwrap());
        assert_eq!(claims["exp"], 1_700_000_000);
    }

    #[test]
    fn argument_exp_injected_when_missing() {
        let token = generate(r#"{"sub":"user-1"}"#, Some(1_750_000_000)).unwrap();
        let claims = decode_segment(token.split('.').nth(1).unwrap());
        assert_eq!(claims["exp"], 1_750_000_000);
    }

    #[test]
    fn no_exp_when_not_provided() {
        let token = generate(r#"{"sub":"user-1"}"#, None).unwrap();
        let claims = decode_segment(token.split('.').nth(1).unwrap());
        assert!(claims.get("exp").is_none());
    }

    #[test]
    fn non_numeric_exp_errors() {
        let err = generate(r#"{"sub":"user-1","exp":"soon"}"#, None).unwrap_err();
        assert!(err.to_string().contains("\"exp\""));
    }

    #[test]
    fn negative_exp_errors() {
        let err = generate(r#"{"sub":"user-1","exp":-5}"#, None).unwrap_err();
        assert!(err.to_string().contains("\"exp\""));
    }

    #[test]
    fn empty_secret_errors() {
        let result = generate_jwt(JwtOptions {
            payload: String::from(r#"{"sub":"user-1"}"#),
            secret: String::new(),
            exp: None,
        });
        assert_eq!(result.unwrap_err(), JwtError::EmptySecret);
    }
}
