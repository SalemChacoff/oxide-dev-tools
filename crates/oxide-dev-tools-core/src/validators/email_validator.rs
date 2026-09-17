//! RFC-compliant email address validation.
//!
//! [`validate_email`] checks an address against two grammars:
//!
//! - [`EmailMode::Standard`] (default) — the RFC 5321 SMTP `Mailbox`
//!   production: dot-string or quoted-string local part, a DNS domain or an
//!   address literal, plus the RFC 5321 length limits (64/255/254 octets)
//!   and the RFC 3696 application restrictions. This is what a mailbox must
//!   satisfy to be deliverable over SMTP.
//! - [`EmailMode::Header`] — the RFC 5322 `addr-spec` production as found in
//!   message headers: comments, folding whitespace, and obsolete `obs-*`
//!   syntax are accepted, and constructs that violate RFC 5321 are reported
//!   as warnings instead of issues.
//!
//! Extensions are supported: SMTPUTF8 (RFC 6531) allows UTF-8 in the local
//! part, and internationalized domains (RFC 5890) are validated by
//! converting each Unicode label to its punycode A-label (RFC 3492) and
//! checking the resulting length, while existing `xn--` labels are verified
//! by decoding them and requiring a round-trip match.
//!
//! The validator performs no network lookups; syntax and lengths are
//! checked locally.

/// Maximum nesting depth accepted for RFC 5322 comments.
const MAX_COMMENT_DEPTH: usize = 32;

// -------- Public API --------

/// Which RFC grammar an address is checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmailMode {
    /// RFC 5321 SMTP mailbox — strict, transport-oriented.
    #[default]
    Standard,
    /// RFC 5322 addr-spec — lenient, message-header-oriented.
    Header,
}

/// Options for [`validate_email`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailOptions {
    /// The address to validate (surrounding whitespace is ignored).
    pub address: String,
    /// Grammar to validate against.
    pub mode: EmailMode,
    /// Accept quoted-string local parts such as `"john doe"@example.com`.
    pub allow_quoted_local: bool,
    /// Accept address literals such as `user@[192.0.2.1]`.
    pub allow_address_literal: bool,
    /// Accept non-ASCII characters (SMTPUTF8 local parts and IDN domains).
    pub allow_utf8: bool,
    /// Require the domain to contain at least two labels.
    pub require_tld: bool,
}

impl Default for EmailOptions {
    fn default() -> Self {
        Self {
            address: String::new(),
            mode: EmailMode::Standard,
            allow_quoted_local: true,
            allow_address_literal: true,
            allow_utf8: true,
            require_tld: false,
        }
    }
}

/// How the domain part was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailDomainKind {
    /// A regular DNS name (possibly internationalized).
    Dns,
    /// An IPv4 address literal such as `[192.0.2.1]`.
    Ipv4Literal,
    /// An IPv6 address literal such as `[IPv6:2001:db8::1]`.
    Ipv6Literal,
    /// A tagged general address literal such as `[tag:value]`.
    GeneralLiteral,
}

/// The result of validating an address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailReport {
    /// Whether the address satisfies the chosen grammar.
    pub valid: bool,
    /// The address as given, without surrounding whitespace.
    pub input: String,
    /// The address with comments, folding whitespace, and domain case
    /// differences removed.
    pub normalized: String,
    /// The local part (keeps quotes for quoted-string locals).
    pub local_part: String,
    /// The domain as written (address literals keep their brackets).
    pub domain: String,
    /// How the domain was written.
    pub domain_kind: EmailDomainKind,
    /// The local part contains non-ASCII characters (SMTPUTF8).
    pub smtputf8: bool,
    /// The domain contains non-ASCII labels (IDN).
    pub idn: bool,
    /// Reasons the address is invalid.
    pub issues: Vec<String>,
    /// Valid but unusual or discouraged constructs.
    pub warnings: Vec<String>,
}

/// Validate an email address and produce a detailed report.
///
/// Validation never fails with an error and never panics: an invalid
/// address is reported via [`EmailReport::valid`] and
/// [`EmailReport::issues`].
pub fn validate_email(options: EmailOptions) -> EmailReport {
    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let trimmed = options.address.trim();
    let mut report = EmailReport {
        valid: false,
        input: trimmed.to_string(),
        normalized: String::new(),
        local_part: String::new(),
        domain: String::new(),
        domain_kind: EmailDomainKind::Dns,
        smtputf8: false,
        idn: false,
        issues: Vec::new(),
        warnings: Vec::new(),
    };

    if trimmed.is_empty() {
        issues.push("address is empty".to_string());
        report.issues = issues;
        report.warnings = warnings;
        return report;
    }

    let chars: Vec<char> = trimmed.chars().collect();
    let mut parser = MailboxParser { src: &chars, pos: 0 };

    let local = parse_local_part(&mut parser, &options, &mut issues, &mut warnings);
    skip_cfws(&mut parser, &options, &mut issues);

    if parser.peek() != Some('@') {
        let mut junk = false;
        while !parser.at_end() && parser.peek() != Some('@') {
            skip_cfws(&mut parser, &options, &mut issues);
            if parser.peek() == Some('@') {
                break;
            }
            parser.bump();
            junk = true;
        }
        if junk {
            issues.push("invalid characters in the local part".to_string());
        }
        if parser.peek() != Some('@') {
            issues.push("missing '@' separator".to_string());
            return finish(report, local, None, &options, issues, warnings);
        }
    }
    parser.bump(); // '@'

    let domain = if parser.at_end() {
        issues.push("domain is empty".to_string());
        None
    } else {
        Some(parse_domain(&mut parser, &options, &mut issues, &mut warnings))
    };

    loop {
        if parser.at_end() {
            break;
        }
        let consumed = skip_cfws(&mut parser, &options, &mut issues);
        if parser.at_end() {
            break;
        }
        if !consumed {
            issues.push("unexpected trailing characters after the domain".to_string());
            break;
        }
    }

    finish(report, local, domain, &options, issues, warnings)
}

// -------- Report assembly --------

fn finish(
    mut report: EmailReport,
    local: Option<LocalPart>,
    domain: Option<ParsedDomain>,
    opts: &EmailOptions,
    mut issues: Vec<String>,
    mut warnings: Vec<String>,
) -> EmailReport {
    if let Some(part) = &local {
        if part.raw_bytes > 64 {
            push_mode(opts, &mut issues, &mut warnings, "local part exceeds 64 octets (RFC 5321)".to_string());
        }
        if part.smtputf8 && !opts.allow_utf8 {
            issues.push("non-ASCII characters in the local part are disabled (SMTPUTF8)".to_string());
        }
    }
    let mut total = 1usize; // the '@'
    if let Some(part) = &local {
        total += part.raw_bytes;
    }
    if let Some(dom) = &domain {
        if dom.raw_bytes > 255 {
            push_mode(opts, &mut issues, &mut warnings, "domain exceeds 255 octets (RFC 5321)".to_string());
        }
        if dom.idn && !opts.allow_utf8 {
            issues.push("non-ASCII characters in the domain are disabled (IDN)".to_string());
        }
        total += dom.raw_bytes;
    }
    if local.is_some() && domain.is_some() && total > 254 {
        push_mode(opts, &mut issues, &mut warnings, "address exceeds 254 octets (RFC 5321)".to_string());
    }

    let local_text = local.as_ref().map_or(String::new(), |part| part.text.clone());
    let domain_text = domain.as_ref().map_or(String::new(), |dom| dom.text.clone());
    let domain_kind = domain.as_ref().map_or(EmailDomainKind::Dns, |dom| dom.kind);
    let normalized = if local.is_some() && domain.is_some() {
        let normalized_domain = if domain_kind == EmailDomainKind::Dns {
            lowercase_text(&domain_text)
        } else {
            domain_text.clone()
        };
        format!("{local_text}@{normalized_domain}")
    } else {
        String::new()
    };

    report.local_part = local_text;
    report.domain = domain_text;
    report.domain_kind = domain_kind;
    report.smtputf8 = local.as_ref().is_some_and(|part| part.smtputf8);
    report.idn = domain.as_ref().is_some_and(|dom| dom.idn);
    report.normalized = normalized;
    report.valid = issues.is_empty();
    report.issues = issues;
    report.warnings = warnings;
    report
}

fn push_mode(opts: &EmailOptions, issues: &mut Vec<String>, warnings: &mut Vec<String>, msg: String) {
    if opts.mode == EmailMode::Standard {
        issues.push(msg);
    } else {
        warnings.push(msg);
    }
}

fn lowercase_text(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

// -------- Parser primitives --------

struct MailboxParser<'src> {
    src: &'src [char],
    pos: usize,
}

impl<'src> MailboxParser<'src> {
    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn peek_n(&self, offset: usize) -> Option<char> {
        self.src.get(self.pos + offset).copied()
    }

    fn bump(&mut self) {
        if self.pos < self.src.len() {
            self.pos += 1;
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.src.len()
    }
}

// -------- Comments and folding whitespace --------

fn skip_fws_raw(parser: &mut MailboxParser<'_>) -> usize {
    let start = parser.pos;
    loop {
        if is_wsp(parser.peek()) {
            parser.bump();
        } else if parser.peek() == Some('\r') && parser.peek_n(1) == Some('\n') && is_wsp(parser.peek_n(2)) {
            parser.bump();
            parser.bump();
            while is_wsp(parser.peek()) {
                parser.bump();
            }
        } else {
            break;
        }
    }
    parser.pos - start
}

fn flag_cfws(opts: &EmailOptions, issues: &mut Vec<String>, flagged: &mut bool) {
    if opts.mode == EmailMode::Standard && !*flagged {
        issues.push("whitespace and comments are not allowed in an SMTP mailbox (RFC 5321)".to_string());
        *flagged = true;
    }
}

fn skip_cfws(parser: &mut MailboxParser<'_>, opts: &EmailOptions, issues: &mut Vec<String>) -> bool {
    let start = parser.pos;
    let mut flagged = false;
    loop {
        match parser.peek() {
            Some('(') => {
                flag_cfws(opts, issues, &mut flagged);
                if opts.mode == EmailMode::Header {
                    parse_comment(parser, issues);
                } else {
                    skip_comment_raw(parser, issues);
                }
            }
            Some(' ' | '\t') => {
                flag_cfws(opts, issues, &mut flagged);
                skip_fws_raw(parser);
            }
            Some('\r') if parser.peek_n(1) == Some('\n') && is_wsp(parser.peek_n(2)) => {
                flag_cfws(opts, issues, &mut flagged);
                skip_fws_raw(parser);
            }
            _ => break,
        }
    }
    parser.pos != start
}

fn parse_comment(parser: &mut MailboxParser<'_>, issues: &mut Vec<String>) {
    parser.bump(); // opening '('
    let mut depth = 0usize;
    loop {
        match parser.peek() {
            None => {
                issues.push("unterminated comment".to_string());
                return;
            }
            Some(')') => {
                parser.bump();
                if depth == 0 {
                    return;
                }
                depth -= 1;
            }
            Some('(') => {
                depth += 1;
                if depth > MAX_COMMENT_DEPTH {
                    issues.push(format!("comment nesting exceeds {MAX_COMMENT_DEPTH} levels"));
                    parser.bump();
                    depth -= 1;
                } else {
                    parser.bump();
                }
            }
            Some('\\') => {
                parser.bump();
                match parser.peek() {
                    Some('\u{20}'..='\u{7e}' | '\t') => {
                        parser.bump();
                    }
                    Some(_) => {
                        issues.push("invalid escape sequence in comment".to_string());
                        parser.bump();
                    }
                    None => {
                        issues.push("unterminated comment".to_string());
                        return;
                    }
                }
            }
            Some(ch) if is_ctext(ch) || matches!(ch, ' ' | '\t' | '\r' | '\n') => {
                parser.bump();
            }
            Some(ch) => {
                issues.push(format!("invalid character '{ch}' in comment"));
                parser.bump();
            }
        }
    }
}

fn skip_comment_raw(parser: &mut MailboxParser<'_>, issues: &mut Vec<String>) {
    parser.bump(); // opening '('
    let mut depth = 1usize;
    while depth > 0 {
        match parser.peek() {
            None => {
                issues.push("unterminated comment".to_string());
                return;
            }
            Some('(') => {
                depth += 1;
                parser.bump();
            }
            Some(')') => {
                depth -= 1;
                parser.bump();
            }
            Some('\\') => {
                parser.bump();
                parser.bump();
            }
            Some(_) => {
                parser.bump();
            }
        }
    }
}

// -------- Local part --------

struct LocalPart {
    text: String,
    raw_bytes: usize,
    smtputf8: bool,
}

fn parse_local_part(
    parser: &mut MailboxParser<'_>,
    opts: &EmailOptions,
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<LocalPart> {
    match opts.mode {
        EmailMode::Standard => parse_local_standard(parser, opts, issues, warnings),
        EmailMode::Header => parse_local_header(parser, opts, issues, warnings),
    }
}

fn parse_local_standard(
    parser: &mut MailboxParser<'_>,
    opts: &EmailOptions,
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<LocalPart> {
    if parser.peek() == Some('"') {
        if !opts.allow_quoted_local {
            issues.push("quoted-string local parts are disabled".to_string());
        }
        let (text, raw_bytes, smtputf8) = parse_quoted_string(parser, opts, issues);
        warnings.push("quoted-string local parts are not accepted by all mail systems".to_string());
        Some(LocalPart {
            text,
            raw_bytes,
            smtputf8,
        })
    } else {
        if parser.peek() == Some('.') {
            issues.push("local part must not start with a dot (RFC 5321)".to_string());
            while parser.peek() == Some('.') {
                parser.bump();
            }
        }
        let (text, raw_bytes, smtputf8) = parse_dot_string(parser, issues);
        if text.is_empty() {
            if parser.peek() == Some('@') || parser.at_end() {
                issues.push("local part is empty".to_string());
            }
            return None;
        }
        Some(LocalPart {
            text,
            raw_bytes,
            smtputf8,
        })
    }
}

fn parse_local_header(
    parser: &mut MailboxParser<'_>,
    opts: &EmailOptions,
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<LocalPart> {
    skip_cfws(parser, opts, issues);
    let mut text = String::new();
    let mut raw_bytes = 0usize;
    let mut smtputf8 = false;
    let mut word_count = 0usize;
    let mut quoted_any = false;
    let mut obsolete = false;

    loop {
        let had_cfws = skip_cfws(parser, opts, issues);
        let more_content = matches!(
            parser.peek(),
            Some(ch) if ch == '.' || ch == '"' || is_atext(ch) || !ch.is_ascii()
        );
        if had_cfws && word_count > 0 && more_content {
            obsolete = true;
        }
        match parser.peek() {
            None | Some('@') => break,
            Some('.') => {
                if word_count == 0 || text.ends_with('.') {
                    issues.push("local part must not contain leading or consecutive dots".to_string());
                }
                parser.bump();
                text.push('.');
                raw_bytes += 1;
            }
            Some('"') => {
                if word_count > 0 && !text.ends_with('.') {
                    issues.push("words in an obsolete local part must be separated by dots".to_string());
                    break;
                }
                let (word, word_bytes, word_smtp) = parse_quoted_string(parser, opts, issues);
                text.push_str(&word);
                raw_bytes += word_bytes;
                smtputf8 |= word_smtp;
                word_count += 1;
                quoted_any = true;
            }
            Some(ch) if is_atext(ch) || !ch.is_ascii() => {
                if word_count > 0 && !text.ends_with('.') {
                    issues.push("words in an obsolete local part must be separated by dots".to_string());
                    break;
                }
                if quoted_any {
                    obsolete = true;
                }
                let (word, word_bytes, word_smtp) = parse_atom_run(parser, issues);
                text.push_str(&word);
                raw_bytes += word_bytes;
                smtputf8 |= word_smtp;
                word_count += 1;
            }
            Some(_) => {
                issues.push("invalid character in the local part".to_string());
                parser.bump();
            }
        }
    }

    if obsolete {
        warnings.push("obsolete local-part syntax (RFC 5322 obs-local-part)".to_string());
    }
    if text.is_empty() {
        if parser.peek() == Some('@') || parser.at_end() {
            issues.push("local part is empty".to_string());
        }
        return None;
    }
    if text.ends_with('.') {
        issues.push("local part must not end with a dot".to_string());
    }
    Some(LocalPart {
        text,
        raw_bytes,
        smtputf8,
    })
}

fn parse_dot_string(parser: &mut MailboxParser<'_>, issues: &mut Vec<String>) -> (String, usize, bool) {
    let (mut text, mut raw_bytes, mut smtputf8) = parse_atom_run(parser, issues);
    loop {
        if parser.peek() != Some('.') {
            break;
        }
        parser.bump();
        raw_bytes += 1;
        let (run, run_bytes, run_smtp) = parse_atom_run(parser, issues);
        if run.is_empty() {
            issues.push("local part must not contain consecutive or trailing dots (RFC 5321)".to_string());
            break;
        }
        text.push('.');
        text.push_str(&run);
        raw_bytes += run_bytes;
        smtputf8 |= run_smtp;
    }
    (text, raw_bytes, smtputf8)
}

fn parse_atom_run(parser: &mut MailboxParser<'_>, issues: &mut Vec<String>) -> (String, usize, bool) {
    let mut text = String::new();
    let mut raw_bytes = 0usize;
    let mut smtputf8 = false;
    while let Some(ch) = parser.peek() {
        if is_atext(ch) {
            text.push(ch);
            raw_bytes += 1;
            parser.bump();
        } else if !ch.is_ascii() {
            if ch.is_control() {
                issues.push("control characters are not allowed in the local part".to_string());
                break;
            }
            text.push(ch);
            raw_bytes += ch.len_utf8();
            smtputf8 = true;
            parser.bump();
        } else {
            break;
        }
    }
    (text, raw_bytes, smtputf8)
}

fn parse_quoted_string(
    parser: &mut MailboxParser<'_>,
    opts: &EmailOptions,
    issues: &mut Vec<String>,
) -> (String, usize, bool) {
    parser.bump(); // opening quote
    let mut text = String::from("\"");
    let mut raw_bytes = 1usize;
    let mut smtputf8 = false;
    let mut terminated = false;

    while let Some(ch) = parser.peek() {
        if ch == '"' {
            parser.bump();
            text.push('"');
            raw_bytes += 1;
            terminated = true;
            break;
        }
        if ch == '\\' {
            parser.bump();
            raw_bytes += 1;
            match parser.peek() {
                Some(esc) if is_quoted_pair_char(esc, opts.mode) => {
                    text.push(esc);
                    raw_bytes += esc.len_utf8();
                    parser.bump();
                }
                Some(esc) => {
                    issues.push("invalid escape sequence in quoted string".to_string());
                    text.push(esc);
                    raw_bytes += esc.len_utf8();
                    parser.bump();
                }
                None => {
                    issues.push("unterminated quoted string".to_string());
                    break;
                }
            }
        } else if ch == ' ' || is_qtext(ch) {
            text.push(ch);
            raw_bytes += 1;
            parser.bump();
        } else if ch == '\t' || ch == '\r' || ch == '\n' {
            if opts.mode == EmailMode::Header {
                let consumed = skip_fws_raw(parser);
                if consumed > 0 {
                    text.push(' ');
                    raw_bytes += consumed;
                } else {
                    issues.push("invalid character in quoted string".to_string());
                    parser.bump();
                }
            } else {
                issues.push("tab/newline is not allowed inside a quoted string (RFC 5321)".to_string());
                parser.bump();
                raw_bytes += 1;
                if ch == '\r' && parser.peek() == Some('\n') {
                    parser.bump();
                    raw_bytes += 1;
                }
            }
        } else if !ch.is_ascii() {
            if ch.is_control() {
                issues.push("control characters are not allowed in the local part".to_string());
                parser.bump();
            } else {
                text.push(ch);
                raw_bytes += ch.len_utf8();
                smtputf8 = true;
                parser.bump();
            }
        } else {
            issues.push(format!("invalid character '{ch}' in quoted string"));
            parser.bump();
        }
    }

    if !terminated {
        issues.push("unterminated quoted string".to_string());
    }
    (text, raw_bytes, smtputf8)
}

// -------- Domain --------

struct ParsedDomain {
    text: String,
    kind: EmailDomainKind,
    raw_bytes: usize,
    idn: bool,
}

fn parse_domain(
    parser: &mut MailboxParser<'_>,
    opts: &EmailOptions,
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> ParsedDomain {
    skip_cfws(parser, opts, issues);
    if parser.peek() == Some('[') {
        parse_address_literal(parser, opts, issues, warnings)
    } else {
        parse_dns_domain(parser, opts, issues, warnings)
    }
}

fn parse_dns_domain(
    parser: &mut MailboxParser<'_>,
    opts: &EmailOptions,
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> ParsedDomain {
    let mut labels: Vec<String> = Vec::new();
    let mut raw_bytes = 0usize;
    let mut idn = false;
    let mut obsolete = false;

    loop {
        if opts.mode == EmailMode::Header {
            let consumed = skip_cfws(parser, opts, issues);
            if consumed && !labels.is_empty() {
                obsolete = true;
            }
        }
        if parser.peek() == Some('.') {
            if labels.is_empty() {
                issues.push("domain must not start with a dot".to_string());
            } else {
                issues.push("domain must not contain consecutive dots".to_string());
            }
            parser.bump();
            raw_bytes += 1;
            continue;
        }
        let (label, label_bytes, non_ascii) = parse_label(parser, issues);
        if label.is_empty() {
            break;
        }
        check_dns_label(&label, non_ascii, opts, issues, warnings);
        labels.push(label);
        raw_bytes += label_bytes;
        idn |= non_ascii;
        if parser.peek() == Some('.') {
            parser.bump();
            raw_bytes += 1;
            if parser.at_end() {
                issues.push("domain must not end with a dot".to_string());
                break;
            }
        } else {
            break;
        }
    }

    if labels.is_empty() {
        issues.push("domain is empty".to_string());
    } else {
        if obsolete {
            warnings.push("obsolete domain syntax (RFC 5322 obs-domain)".to_string());
        }
        if let Some(last_label) = labels.last() {
            if last_label.bytes().all(|byte| byte.is_ascii_digit()) {
                push_mode(opts, issues, warnings, "top-level domain must not be all-numeric (RFC 3696)".to_string());
            }
            if last_label.chars().count() == 1 {
                warnings.push("single-character top-level domains do not exist in the DNS root".to_string());
            }
        }
        if labels.len() == 1 {
            if opts.require_tld {
                issues.push("domain must contain at least two labels".to_string());
            } else {
                warnings.push("single-label domains may not be deliverable".to_string());
            }
        }
    }

    ParsedDomain {
        text: labels.join("."),
        kind: EmailDomainKind::Dns,
        raw_bytes,
        idn,
    }
}

fn parse_label(parser: &mut MailboxParser<'_>, issues: &mut Vec<String>) -> (String, usize, bool) {
    let mut label = String::new();
    let mut bytes = 0usize;
    let mut non_ascii = false;
    while let Some(ch) = parser.peek() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            label.push(ch);
            bytes += 1;
            parser.bump();
        } else if !ch.is_ascii() {
            if ch.is_control() {
                issues.push("control characters are not allowed in the domain".to_string());
                break;
            }
            label.push(ch);
            bytes += ch.len_utf8();
            non_ascii = true;
            parser.bump();
        } else {
            break;
        }
    }
    (label, bytes, non_ascii)
}

fn check_dns_label(
    label: &str,
    non_ascii: bool,
    opts: &EmailOptions,
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    if non_ascii {
        let chars: Vec<char> = label.chars().collect();
        if !u_label_ok(&chars) {
            push_mode(
                opts,
                issues,
                warnings,
                format!("domain label '{label}' contains characters not allowed by IDNA"),
            );
            return;
        }
        if let Some(a_label) = idn_a_label(label) {
            if a_label.len() > 63 {
                push_mode(opts, issues, warnings, format!("domain label '{label}' exceeds 63 octets in punycode form"));
            }
        }
    } else {
        if label.len() > 63 {
            push_mode(opts, issues, warnings, format!("domain label '{label}' exceeds 63 octets"));
        }
        if label.starts_with('-') || label.ends_with('-') {
            push_mode(opts, issues, warnings, format!("domain label '{label}' must not start or end with a hyphen"));
        }
        if label.len() >= 4 && label[..4].eq_ignore_ascii_case("xn--") {
            check_a_label(label, opts, issues, warnings);
        }
    }
}

fn u_label_ok(chars: &[char]) -> bool {
    !chars.is_empty()
        && chars.iter().all(|ch| ch.is_alphanumeric() || *ch == '-')
        && chars.first() != Some(&'-')
        && chars.last() != Some(&'-')
}

fn check_a_label(label: &str, opts: &EmailOptions, issues: &mut Vec<String>, warnings: &mut Vec<String>) {
    let payload = &label[4..];
    let mut push = |msg: String| push_mode(opts, issues, warnings, msg);
    if payload.is_empty() {
        push("invalid punycode A-label".to_string());
        return;
    }
    if !payload.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-') {
        push("invalid punycode A-label".to_string());
        return;
    }
    let lowered = payload.to_ascii_lowercase();
    let Some(decoded) = punycode_decode(&lowered) else {
        push("invalid punycode A-label".to_string());
        return;
    };
    if !u_label_ok(&decoded) {
        push("invalid punycode A-label".to_string());
        return;
    }
    let Some(encoded) = punycode_encode(&decoded) else {
        push("invalid punycode A-label".to_string());
        return;
    };
    if encoded != lowered {
        push("invalid punycode A-label (does not round-trip)".to_string());
    }
}

fn idn_a_label(label: &str) -> Option<String> {
    let chars: Vec<char> = label.chars().collect();
    let encoded = punycode_encode(&chars)?;
    Some(format!("xn--{encoded}"))
}

// -------- Address literals --------

fn parse_address_literal(
    parser: &mut MailboxParser<'_>,
    opts: &EmailOptions,
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> ParsedDomain {
    if !opts.allow_address_literal {
        issues.push("address literals are disabled".to_string());
    }
    parser.bump(); // opening '['
    let mut content = String::new();
    let mut terminated = false;

    while let Some(ch) = parser.peek() {
        if ch == ']' {
            parser.bump();
            terminated = true;
            break;
        }
        if ch == '\\' {
            if opts.mode == EmailMode::Header {
                parser.bump();
                match parser.peek() {
                    Some(esc @ ('\u{20}'..='\u{7e}' | '\t')) => {
                        content.push(esc);
                        parser.bump();
                    }
                    Some(_) => {
                        issues.push("invalid escape sequence in address literal".to_string());
                        parser.bump();
                    }
                    None => {
                        issues.push("unterminated address literal".to_string());
                        break;
                    }
                }
            } else {
                issues.push("escape sequences are not allowed in an address literal (RFC 5321)".to_string());
                parser.bump();
            }
        } else if matches!(ch, ' ' | '\t' | '\r' | '\n') {
            if opts.mode == EmailMode::Header {
                skip_fws_raw(parser);
            } else {
                issues.push("whitespace is not allowed in an address literal (RFC 5321)".to_string());
                parser.bump();
            }
        } else if is_dtext(ch) {
            content.push(ch);
            parser.bump();
        } else {
            issues.push(format!("invalid character '{ch}' in address literal"));
            parser.bump();
        }
    }

    if !terminated {
        issues.push("unterminated address literal".to_string());
    }
    classify_address_literal(&content, issues, warnings)
}

fn classify_address_literal(content: &str, issues: &mut Vec<String>, warnings: &mut Vec<String>) -> ParsedDomain {
    if content.is_empty() {
        issues.push("address literal is empty".to_string());
    }
    let kind = if is_ipv4_literal(content) {
        EmailDomainKind::Ipv4Literal
    } else if content.len() >= 5 && content[..5].eq_ignore_ascii_case("ipv6:") {
        if parse_ipv6(&content[5..]).is_ok() {
            EmailDomainKind::Ipv6Literal
        } else {
            issues.push("invalid IPv6 address literal".to_string());
            EmailDomainKind::Ipv6Literal
        }
    } else if let Some((tag, value)) = content.split_once(':') {
        if !is_standardized_tag(tag) || value.is_empty() {
            issues.push("invalid address literal".to_string());
            EmailDomainKind::GeneralLiteral
        } else {
            EmailDomainKind::GeneralLiteral
        }
    } else {
        issues.push("invalid address literal".to_string());
        EmailDomainKind::GeneralLiteral
    };
    if kind == EmailDomainKind::GeneralLiteral {
        warnings.push("general address literals are rarely supported by mail systems".to_string());
    }
    ParsedDomain {
        text: format!("[{content}]"),
        kind,
        raw_bytes: content.len() + 2,
        idn: false,
    }
}

fn is_standardized_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        && tag.chars().last().is_some_and(|ch| ch.is_ascii_alphanumeric())
}

fn is_ipv4_literal(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|part| {
        !part.is_empty()
            && part.len() <= 3
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && part.parse::<u16>().is_ok_and(|value| value <= 255)
    })
}

// -------- IPv6 address literals --------

fn parse_ipv6(text: &str) -> Result<(), ()> {
    match text.split_once("::") {
        Some((head, tail)) => {
            if tail.contains("::") {
                return Err(());
            }
            let left = count_ipv6_head(head)?;
            let right = count_ipv6_tail(tail)?;
            if left + right > 7 { Err(()) } else { Ok(()) }
        }
        None => {
            if count_ipv6_full(text)? == 8 {
                Ok(())
            } else {
                Err(())
            }
        }
    }
}

fn count_ipv6_head(head: &str) -> Result<usize, ()> {
    if head.is_empty() {
        return Ok(0);
    }
    let mut count = 0usize;
    for group in head.split(':') {
        if !is_h16(group) {
            return Err(());
        }
        count += 1;
    }
    Ok(count)
}

fn count_ipv6_tail(tail: &str) -> Result<usize, ()> {
    if tail.is_empty() {
        return Ok(0);
    }
    let groups: Vec<&str> = tail.split(':').collect();
    count_ipv6_groups(&groups)
}

fn count_ipv6_full(text: &str) -> Result<usize, ()> {
    let groups: Vec<&str> = text.split(':').collect();
    if groups.len() > 8 {
        return Err(());
    }
    count_ipv6_groups(&groups)
}

fn count_ipv6_groups(groups: &[&str]) -> Result<usize, ()> {
    let mut count = 0usize;
    for (index, group) in groups.iter().enumerate() {
        if group.contains('.') {
            if index != groups.len() - 1 || !is_ipv4_literal(group) {
                return Err(());
            }
            count += 2;
        } else if is_h16(group) {
            count += 1;
        } else {
            return Err(());
        }
    }
    Ok(count)
}

fn is_h16(group: &str) -> bool {
    !group.is_empty() && group.len() <= 4 && group.bytes().all(|byte| byte.is_ascii_hexdigit())
}

// -------- IDN punycode (RFC 3492) --------

const PUNY_BASE: u32 = 36;
const PUNY_TMIN: u32 = 1;
const PUNY_TMAX: u32 = 26;
const PUNY_SKEW: u32 = 38;
const PUNY_DAMP: u32 = 700;
const PUNY_INITIAL_BIAS: u32 = 72;
const PUNY_INITIAL_N: u32 = 128;

fn puny_adapt(delta: u32, num_points: u32, first_time: bool) -> u32 {
    let mut adapted = if first_time { delta / PUNY_DAMP } else { delta / 2 };
    adapted += adapted / num_points;
    let mut shift = 0u32;
    while adapted > ((PUNY_BASE - PUNY_TMIN) * PUNY_TMAX) / 2 {
        adapted /= PUNY_BASE - PUNY_TMIN;
        shift += PUNY_BASE;
    }
    shift + (((PUNY_BASE - PUNY_TMIN + 1) * adapted) / (adapted + PUNY_SKEW))
}

fn puny_encode_digit(value: u32) -> char {
    if value < 26 {
        char::from(b'a' + value as u8)
    } else {
        char::from(b'0' + (value - 26) as u8)
    }
}

fn puny_decode_digit(byte: u8) -> Option<u32> {
    match byte {
        b'a'..=b'z' => Some(u32::from(byte - b'a')),
        b'0'..=b'9' => Some(u32::from(byte - b'0') + 26),
        _ => None,
    }
}

fn punycode_encode(input: &[char]) -> Option<String> {
    let mut output = String::new();
    let mut basic_count = 0usize;
    for ch in input {
        if ch.is_ascii() {
            output.push(*ch);
            basic_count += 1;
        }
    }
    if basic_count > 0 {
        output.push('-');
    }
    let mut handled = basic_count;
    let mut nval = PUNY_INITIAL_N;
    let mut delta = 0u32;
    let mut bias = PUNY_INITIAL_BIAS;

    while handled < input.len() {
        let mut min_cp = u32::MAX;
        for ch in input {
            let codepoint = *ch as u32;
            if codepoint >= nval && codepoint < min_cp {
                min_cp = codepoint;
            }
        }
        if min_cp == u32::MAX {
            return None;
        }
        let step = (min_cp - nval).checked_mul(handled as u32 + 1)?;
        delta = delta.checked_add(step)?;
        nval = min_cp;
        for ch in input {
            let codepoint = *ch as u32;
            if codepoint < nval {
                delta = delta.checked_add(1)?;
            }
            if codepoint == nval {
                let mut remainder = delta;
                let mut shift = PUNY_BASE;
                loop {
                    let threshold = if shift <= bias {
                        PUNY_TMIN
                    } else if shift >= bias + PUNY_TMAX {
                        PUNY_TMAX
                    } else {
                        shift - bias
                    };
                    if remainder < threshold {
                        break;
                    }
                    output.push(puny_encode_digit(threshold + ((remainder - threshold) % (PUNY_BASE - threshold))));
                    remainder = (remainder - threshold) / (PUNY_BASE - threshold);
                    shift += PUNY_BASE;
                }
                output.push(puny_encode_digit(remainder));
                bias = puny_adapt(delta, handled as u32 + 1, handled == basic_count);
                delta = 0;
                handled += 1;
            }
        }
        delta = delta.checked_add(1)?;
        nval = nval.checked_add(1)?;
    }
    Some(output)
}

fn punycode_decode(input: &str) -> Option<Vec<char>> {
    let bytes = input.as_bytes();
    let mut output: Vec<char> = Vec::new();
    let mut index = 0usize;

    if let Some(last_delim) = bytes.iter().rposition(|byte| *byte == b'-') {
        for byte in &bytes[..last_delim] {
            if !byte.is_ascii() {
                return None;
            }
            output.push(char::from(*byte));
        }
        index = last_delim + 1;
    }

    let mut nval = PUNY_INITIAL_N;
    let mut delta = 0u32;
    let mut bias = PUNY_INITIAL_BIAS;

    while index < bytes.len() {
        let old_delta = delta;
        let mut weight = 1u32;
        let mut shift = PUNY_BASE;
        loop {
            let digit = puny_decode_digit(*bytes.get(index)?)?;
            index += 1;
            delta = delta.checked_add(digit.checked_mul(weight)?)?;
            let threshold = if shift <= bias {
                PUNY_TMIN
            } else if shift >= bias + PUNY_TMAX {
                PUNY_TMAX
            } else {
                shift - bias
            };
            if digit < threshold {
                break;
            }
            weight = weight.checked_mul(PUNY_BASE - threshold)?;
            shift += PUNY_BASE;
        }
        let out_len = output.len() as u32 + 1;
        bias = puny_adapt(delta - old_delta, out_len, old_delta == 0);
        nval = nval.checked_add(delta / out_len)?;
        delta %= out_len;
        let insert_at = usize::try_from(delta).ok()?;
        if insert_at > output.len() {
            return None;
        }
        output.insert(insert_at, char::from_u32(nval)?);
        delta += 1;
    }
    Some(output)
}

// -------- Character classes --------

fn is_atext(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || "!#$%&'*+-/=?^_`{|}~".contains(ch)
}

fn is_qtext(ch: char) -> bool {
    matches!(ch, '\u{21}'..='\u{7e}') && ch != '"' && ch != '\\'
}

fn is_dtext(ch: char) -> bool {
    matches!(ch, '\u{21}'..='\u{5a}' | '\u{5e}'..='\u{7e}')
}

fn is_ctext(ch: char) -> bool {
    matches!(ch, '\u{21}'..='\u{27}' | '\u{2a}'..='\u{5b}' | '\u{5d}'..='\u{7e}')
}

fn is_wsp(ch: Option<char>) -> bool {
    matches!(ch, Some(' ') | Some('\t'))
}

fn is_quoted_pair_char(ch: char, mode: EmailMode) -> bool {
    matches!(ch, '\u{20}'..='\u{7e}') || (mode == EmailMode::Header && ch == '\t')
}

// -------- Tests --------

#[cfg(test)]
mod tests {
    use super::*;

    fn check(address: &str) -> EmailReport {
        validate_email(EmailOptions {
            address: address.to_string(),
            ..Default::default()
        })
    }

    fn check_header(address: &str) -> EmailReport {
        validate_email(EmailOptions {
            address: address.to_string(),
            mode: EmailMode::Header,
            ..Default::default()
        })
    }

    #[test]
    fn valid_standard_addresses() {
        let valid = [
            "user@example.com",
            "first.last@example.co.uk",
            "user+tag@example.com",
            "user_name@example.com",
            "user-name@example.com",
            "user123@example.com",
            "o'brien@example.com",
            "user@sub.example-domain.com",
            "user@localhost",
            "User@Example.COM",
            "\"john doe\"@example.com",
            "\"john\\\"doe\"@example.com",
            "\"a b\"@example.com",
            "user@[192.0.2.1]",
            "user@[IPv6:::1]",
            "user@[IPv6:2001:db8::1]",
            "user@münchen.de",
            "用户@例子.广告",
            "δοκιμή@example.com",
            "user@xn--bcher-kva.de",
            "user@xn--tda.de",
        ];
        for address in valid {
            let report = check(address);
            assert!(report.valid, "{address}: {:?}", report.issues);
        }
    }

    #[test]
    fn invalid_standard_addresses() {
        let cases = [
            ("", "address is empty"),
            ("plain", "missing '@'"),
            ("@example.com", "local part is empty"),
            ("user@", "domain is empty"),
            ("user@@example.com", "domain is empty"),
            ("a..b@example.com", "consecutive"),
            (".user@example.com", "start with a dot"),
            ("user.@example.com", "consecutive"),
            ("user name@example.com", "whitespace and comments"),
            ("user(comment)@example.com", "whitespace and comments"),
            ("user@exa mple.com", "whitespace and comments"),
            ("user@-example.com", "hyphen"),
            ("user@example-.com", "hyphen"),
            ("user@example.123", "all-numeric"),
            ("user@[999.1.1.1]", "invalid address literal"),
            ("user@[IPv6:1::2::3]", "invalid IPv6"),
            ("user@[IPv6:::1.2.3.300]", "invalid IPv6"),
            ("user@[tag:]", "invalid address literal"),
            ("user@[IPv6:nothex::1]", "invalid IPv6"),
            ("user@exa!mple.com", "unexpected trailing"),
            ("user@xn--abc", "punycode"),
            ("user@xn--", "punycode"),
            ("\u{7f}@example.com", "invalid characters"),
            ("user\u{80}@example.com", "control"),
            ("\"unterminated@example.com", "unterminated quoted string"),
            ("\"a\tb\"@example.com", "tab/newline"),
            ("user@example.com.", "end with a dot"),
        ];
        for (address, expected) in cases {
            let report = check(address);
            assert!(!report.valid, "{address} should be invalid");
            assert!(
                report.issues.iter().any(|issue| issue.contains(expected)),
                "{address}: expected '{expected}' in {:?}",
                report.issues
            );
        }
    }

    #[test]
    fn header_mode_accepts_comments_and_folding() {
        let valid = [
            ("user (comment) @example.com", "user@example.com"),
            ("user@example.com (comment)", "user@example.com"),
            ("user@ example.com", "user@example.com"),
            ("(leading) user@example.com", "user@example.com"),
            ("user . name@example.com", "user.name@example.com"),
            ("user@exa. (c) mple.com", "user@exa.mple.com"),
        ];
        for (address, normalized) in valid {
            let report = check_header(address);
            assert!(report.valid, "{address}: {:?}", report.issues);
            assert_eq!(report.normalized, normalized, "{address}");
        }
    }

    #[test]
    fn header_mode_rejects_unquoted_spaces_between_words() {
        let report = check_header("john doe@example.com");
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("separated by dots")), "{:?}", report.issues);
    }

    #[test]
    fn header_mode_demotes_rfc5321_limits_to_warnings() {
        let long_local = format!("{}@example.com", "a".repeat(65));
        let report = check_header(&long_local);
        assert!(report.valid, "{:?}", report.issues);
        assert!(report.warnings.iter().any(|warning| warning.contains("64 octets")));
    }

    #[test]
    fn header_mode_collapses_folding_whitespace_in_quoted_strings() {
        let report = check_header("\"john\t doe\"@example.com");
        assert!(report.valid, "{:?}", report.issues);
        assert_eq!(report.normalized, "\"john doe\"@example.com");
    }

    #[test]
    fn length_limits() {
        let long_local = format!("{}@example.com", "a".repeat(65));
        let report = check(&long_local);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("64 octets")));

        let long_label = format!("user@{}example.com", "a".repeat(64));
        let report = check(&long_label);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("63 octets")));

        let long_total = format!("{}@{}.com", "a".repeat(64), "b".repeat(200));
        let report = check(&long_total);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("254 octets")));
    }

    #[test]
    fn flags_reject_disabled_constructs() {
        let quoted = EmailOptions {
            address: "\"john\"@example.com".to_string(),
            allow_quoted_local: false,
            ..Default::default()
        };
        let report = validate_email(quoted);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("quoted")));

        let literal = EmailOptions {
            address: "user@[192.0.2.1]".to_string(),
            allow_address_literal: false,
            ..Default::default()
        };
        let report = validate_email(literal);
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("address literals are disabled"))
        );

        let unicode = EmailOptions {
            address: "用户@例子.广告".to_string(),
            allow_utf8: false,
            ..Default::default()
        };
        let report = validate_email(unicode);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("IDN")));

        let smtputf8 = EmailOptions {
            address: "δοκιμή@example.com".to_string(),
            allow_utf8: false,
            ..Default::default()
        };
        let report = validate_email(smtputf8);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("SMTPUTF8")));

        let tld = EmailOptions {
            address: "user@localhost".to_string(),
            require_tld: true,
            ..Default::default()
        };
        let report = validate_email(tld);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("two labels")));
    }

    #[test]
    fn normalized_forms() {
        let report = check("User@Example.COM");
        assert_eq!(report.normalized, "User@example.com");
        assert_eq!(report.local_part, "User");
        assert_eq!(report.domain, "Example.COM");
        assert_eq!(report.domain_kind, EmailDomainKind::Dns);
        assert!(!report.idn);

        let quoted = check("\"John Doe\"@Example.com");
        assert_eq!(quoted.normalized, "\"John Doe\"@example.com");

        let literal = check("user@[IPv6:2001:DB8::1]");
        assert_eq!(literal.domain_kind, EmailDomainKind::Ipv6Literal);
        assert_eq!(literal.normalized, "user@[IPv6:2001:DB8::1]");
    }

    #[test]
    fn report_parts_and_warnings() {
        let report = check("user@localhost");
        assert!(report.valid);
        assert!(report.warnings.iter().any(|warning| warning.contains("single-label")));

        let quoted = check("\"john doe\"@example.com");
        assert!(quoted.valid);
        assert!(quoted.warnings.iter().any(|warning| warning.contains("quoted")));

        let idn = check("用户@例子.广告");
        assert!(idn.idn);
        assert!(idn.smtputf8);

        let literal = check("user@[tag:value]");
        assert!(literal.valid);
        assert!(
            literal
                .warnings
                .iter()
                .any(|warning| warning.contains("rarely supported"))
        );
    }

    #[test]
    fn empty_default_options_report_invalid() {
        let report = validate_email(EmailOptions::default());
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.contains("empty")));
    }

    #[test]
    fn punycode_encodes_rfc3492_vectors() {
        let cases = [
            ("bücher", "bcher-kva"),
            ("München", "Mnchen-3ya"),
            ("☃", "n3h"),
            ("mañana", "maana-pta"),
            ("abæcdöef", "abcdef-qua4k"),
            ("3年B組金八先生", "3B-ww4c5e180e575a65lsy2b"),
            ("安室奈美恵-with-SUPER-MONKEYS", "-with-SUPER-MONKEYS-pc58ag80a8qai00g7n9n"),
        ];
        for (input, expected) in cases {
            let chars: Vec<char> = input.chars().collect();
            let encoded = punycode_encode(&chars).expect("should encode");
            assert_eq!(encoded, expected, "{input}");
        }
    }

    #[test]
    fn punycode_round_trips() {
        let inputs = [
            "bücher",
            "München",
            "☃",
            "mañana",
            "abæcdöef",
            "用户",
            "δοκιμή",
            "例子.广告",
            "安室奈美恵-with-SUPER-MONKEYSスーパーモンキーズ",
        ];
        for input in inputs {
            let chars: Vec<char> = input.chars().collect();
            let encoded = punycode_encode(&chars).expect("should encode");
            let decoded = punycode_decode(&encoded).expect("should decode");
            assert_eq!(decoded, chars, "{input}");
        }
    }

    #[test]
    fn ipv6_literals() {
        let valid = [
            "::",
            "::1",
            "1::",
            "1::2",
            "::ffff:192.168.0.1",
            "2001:db8::8a2e:370:7334",
            "fe80::",
            "1:2:3:4:5:6:7:8",
            "1:2:3:4:5:6:1.2.3.4",
            "::1.2.3.4",
        ];
        for address in valid {
            assert!(parse_ipv6(address).is_ok(), "{address}");
        }
        let invalid = [
            "1::2::3",
            ":::",
            "1:2:3:4:5:6:7:8::",
            "1:2:3:4:5:6:7:1.2.3.4",
            "g::1",
            "12345::1",
            "::1.2.3.300",
            "1.2.3.4",
            "::1.2.3.4.5",
            "",
        ];
        for address in invalid {
            assert!(parse_ipv6(address).is_err(), "{address}");
        }
    }
}
