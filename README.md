# oxide-dev-tools

A fast, unified CLI toolkit for developers — generators, validators, comparators, text utilities, codecs, and structural analyzers, all in one binary.

> **Status:** Early development. New tools are being added regularly.

---

## Features

### ✅ Currently available

| Category | Tools |
|---|---|
| **ID Generator** (`oxide gen id`) | UUID v1–v8, ULID, NanoID |
| **Key Generator** (`oxide gen key`) | Passwords, Tokens, JWTs (HS256) |
| **Data Generator** (`oxide gen lorem`) | Lorem ipsum words, sentences, paragraphs |
| **Data Generator** (`oxide gen fake`) | Fake personas, names, emails, phones, addresses, companies |
| **Sample File Generator** (`oxide gen sample`) | PDF, PNG, JPG files with exact sizes, dimensions, colors, tamper variants |
| **Codecs** (`oxide codec`) | Base64 encode/decode (standard and URL-safe), Hex encode/decode, URL encode/decode |
| **Converters** (`oxide convert`) | Timestamp ↔ Unix/ISO 8601/RFC 2822/human-readable, with units, precision, and timezones; unit conversion (data storage, data rate, length, time, mass); JSON ↔ YAML ↔ XML document conversion (inline text or file input, stdout or file output)

### 🚧 Planned / In progress

| Category | Description |
|---|---|
| **Validators** | Validate emails, URLs, IPs, UUIDs, JSON, YAML, credit cards, and more |
| **Comparators** | Diff text, JSON, directories; semantic version compare |
| **Text Utilities** | Case conversion, slugify, count (words/lines/chars), truncate, encode/decode |
| **Codecs** | PEM/PFX parsing, ZIP compression |
| **Converters** | Units, JSON ↔ YAML, color formats |
| **Data Generator** | Fake personas, names, emails, phones, addresses, companies; sample CSV/JSON data |
| **File Generator** | Boilerplate scaffolding (gitignore, license, Dockerfile, CI configs) |
| **Structural Analyzers** | Analyze JSON/YAML/XML structure, file tree, dependency graph |

---

## Installation

### From source

```bash
# Clone the repository
git clone https://github.com/SalemChacoff/oxide-dev-tools.git
cd oxide-dev-tools

# Build the binary
cargo build --release

# The binary is at: ./target/release/oxide
# Optional: copy it to your PATH
cp ./target/release/oxide ~/.cargo/bin/

# Or install directly from the workspace
cargo install --path crates/oxide-dev-tools-cli
```

### Requirements

- [Rust](https://rustup.rs/) 2024 edition or later

---

## Usage

```bash
oxide <tool> [subcommand] [options]
```

### Examples

```bash
# Generate a UUID v4
oxide gen id uuidv4

# Generate a UUID v7 with a specific date
oxide gen id uuidv7 2026-06-07

# Generate a NanoID
oxide gen id nanoid

# Generate a ULID
oxide gen id ulid

# Generate a password
oxide gen key pass

# Generate a random token (hex, 32 bytes)
oxide gen key token

# Generate a base64 token
oxide gen key token --encoding base64

# Generate an HS256 JWT from a JSON payload (expires in 1 hour)
oxide gen key jwt '{"sub":"user-1"}' --secret my-secret --exp 1h

# JWT with an explicit expiry claim (payload "exp" wins over --exp)
oxide gen key jwt '{"sub":"user-1","exp":1750000000}' --secret my-secret

> Note: on Windows shells, JSON quoting differs — e.g. `oxide gen key jwt "{\"sub\":\"user-1\"}" --secret my-secret`.

# Generate 10 lorem ipsum words
oxide gen lorem words

# Generate 20 lorem ipsum words starting with the classic opener
oxide gen lorem words --length 20 --start

# Generate 3 lorem ipsum sentences (4–12 words each)
oxide gen lorem sentences

# Generate 2 paragraphs of 5 sentences each, starting with the classic opener
oxide gen lorem paragraphs --length 2 --sentences-per-paragraph 5 --start

# Generate a full fake persona (name, email, phone, address, company, job)
oxide gen fake person

# Generate a random first name, surname, and full name
oxide gen fake name
oxide gen fake surname
oxide gen fake fullname

# Generate a fake email address and a US phone number
oxide gen fake email
oxide gen fake phone

# Generate a fake street address (street, city, country)
oxide gen fake address

# Generate a random city, country, company, job title, and username
oxide gen fake city
oxide gen fake country
oxide gen fake company
oxide gen fake job
oxide gen fake username

# Generate 10 emails, one per line
oxide gen fake email --count 10

# Generate 3 persona cards, separated by blank lines
oxide gen fake person --count 3

# Generate a 5kb PDF with custom text
oxide gen sample pdf --size 5kb --text "invoice #1"

# Generate a 5MB red PNG (dimensions auto-picked to fit the size)
oxide gen sample png --size 5mb --color red --output ./upload.png

# Generate a 2MB JPEG and write it with a .txt extension (extension tests)
oxide gen sample jpg --size 2mb --wrong-ext txt

# Generate a PDF with zeroed magic bytes (magic-byte tests)
oxide gen sample pdf --size 5kb --tamper magic

# Stream a sample file straight into a multipart upload
oxide gen sample pdf --size 5kb --output - | curl -F "file=@-;filename=sample.pdf" http://localhost:8080/upload

# Encode text as base64
oxide codec base64 encode "hello world"

# Decode base64 back into text
oxide codec base64 decode "aGVsbG8gd29ybGQ="

# URL-safe, unpadded base64 (JWT style)
oxide codec base64 encode "hello" --url
oxide codec base64 decode "aGVsbG8" --url

# Encode text as hex
oxide codec hex encode "hello"

# Uppercase hex output
oxide codec hex encode "hello" --upper

# Decode hex back into text (case-insensitive, whitespace tolerated)
oxide codec hex decode "68656c6c6f"
oxide codec hex decode "68 65 6c 6c 6f"

# Encode text as a URL component (RFC 3986 percent-encoding)
oxide codec url encode "hello world"

# Form encoding: space becomes `+`
oxide codec url encode "hello world" --form

# Decode a percent-encoded URL component back into text
oxide codec url decode "hello%20world"
oxide codec url decode "hello+world" --form

# Convert a Unix timestamp to ISO 8601 (format auto-detected)
oxide convert timestamp 1750000000

# Milliseconds, microseconds, and nanoseconds are auto-detected by digit count
oxide convert timestamp 1750000000000
oxide convert timestamp 1750000000000000
oxide convert timestamp 1750000000000000000

# Convert an ISO 8601 date to Unix seconds (offsets are honored)
oxide convert timestamp 2026-06-07T12:34:56Z
oxide convert timestamp 2026-06-07T12:34:56+02:00

# Date-only and space-separated inputs are accepted
oxide convert timestamp 2026-06-07
oxide convert timestamp "2026-06-07 12:34:56"

# Pick the output format: unix, iso, rfc2822, or human
oxide convert timestamp 2026-06-07T12:34:56Z --to rfc2822
oxide convert timestamp 2026-06-07T12:34:56Z --to human

# Unix output in milliseconds with a fixed fractional precision
oxide convert timestamp 2026-06-07T12:34:56.123456789Z --to unix --unit ms --precision 3

# Render dates in another timezone (IANA names, ±HH:MM, or local)
oxide convert timestamp 1750000000 --zone Europe/Berlin
oxide convert timestamp 1750000000 --zone +05:30
oxide convert timestamp 1750000000 --zone local

# Force the input format when auto-detection is ambiguous
oxide convert timestamp 1750000000 --from unix --to iso

# Invalid dates are rejected with a specific reason
oxide convert timestamp 2026-02-30

# Convert data storage sizes (SI and IEC prefixes; lowercase b = bit, uppercase B = byte)
oxide convert storage 1.5 gB --to mib
oxide convert storage 1.5gB --to mib          # unit glued to the value
oxide convert storage 1 kb --to KiB

# Convert data rates (per second)
oxide convert rate 100 mbit/s --to mb/s
oxide convert rate 8 mbps --to mB/s

# Convert lengths (metric and imperial)
oxide convert length 5 km --to mi
oxide convert length 12 in --to cm

# Convert time durations (months/years are calendar-aware via --anchor)
oxide convert time 90 min --to h
oxide convert time 1 y --to d --anchor 2020-01-01

# Convert masses (metric and imperial)
oxide convert mass 200 lb --to kg
oxide convert mass 1 oz --to g

# Convert JSON to YAML (text in, text out)
oxide convert json2yaml '{"name":"oxide","versions":[1,2]}'

# Convert YAML from a file to JSON, written to a file
oxide convert yaml2json ./config.yaml --output ./config.json

# Convert JSON to XML with a custom root element and pretty output
oxide convert json2xml '{"a":1,"b":[true,null]}' --root-name data --pretty

# Convert XML to JSON from a file (--input-file errors on missing files)
oxide convert xml2json ./report.xml --input-file --pretty

# Convert between YAML and XML in both directions
oxide convert yaml2xml 'items: [one, two]'
oxide convert xml2yaml '<root><items>one</items><items>two</items></root>'

# List every unit in a category
oxide convert storage --list
oxide convert length --list

# Show help
oxide --help
oxide gen --help
oxide gen id --help
oxide gen lorem --help
oxide gen fake --help
oxide gen sample --help
oxide codec --help
oxide codec base64 --help
oxide codec hex --help
oxide codec url --help
oxide convert --help
oxide convert timestamp --help
oxide convert storage --help
oxide convert rate --help
oxide convert length --help
oxide convert time --help
oxide convert mass --help
oxide convert json2yaml --help
oxide convert yaml2json --help
oxide convert json2xml --help
oxide convert xml2json --help
oxide convert yaml2xml --help
oxide convert xml2yaml --help
```

---

## Project Structure

```
oxide-dev-tools/
├── crates/
│   ├── oxide-dev-tools-core/   # Core library — all logic lives here
│   │   └── src/
│       │       ├── codecs/          # Codec implementations (base64, hex, URL, ...)
│       │       │   ├── base64_codec.rs
│       │       │   ├── hex_codec.rs
│       │       │   ├── url_codec.rs
│       │       │   └── mod.rs
│       │       ├── converters/      # Converter implementations (timestamp, units, ...)
│       │       │   ├── timestamp_converter.rs # Unix ↔ ISO 8601 ↔ RFC 2822 ↔ human-readable
│       │       │   ├── unit_converter.rs      # Data storage/rate, length, time, mass conversions
│       │       │   ├── doc_converter.rs       # JSON ↔ YAML ↔ XML document conversion
│       │       │   └── mod.rs
│       │       ├── generators/
│       │       │   ├── fake_generator.rs  # Fake personas, names, emails, phones, addresses
│       │       │   ├── id_generator.rs   # UUID v1–v8, ULID, NanoID + planned IDs
│       │       │   ├── jwt_generator.rs  # JWT (HS256) generation
│       │       │   ├── key_generator.rs  # Password, token generation
│       │       │   ├── lorem_generator.rs # Lorem ipsum words, sentences, paragraphs
│       │       │   ├── sample_file_generator.rs # Sample PDF/PNG/JPG files with exact sizes
│       │       │   └── mod.rs
│   │       └── lib.rs
│   └── oxide-dev-tools-cli/    # CLI binary — clap-based argument parsing
│       └── src/
│   │           ├── codecs/          # CLI wrappers for codecs
│   │           │   ├── base64_codec.rs
│   │           │   ├── hex_codec.rs
│   │           │   ├── url_codec.rs
│   │           │   └── mod.rs
│   │           ├── converters/      # CLI wrappers for converters
│   │           │   ├── timestamp_converter.rs
│   │           │   ├── unit_converter.rs
│   │           │   ├── doc_converter.rs
│   │           │   └── mod.rs
│   │           ├── generators/     # CLI subcommand wrappers for generators
│   │           │   ├── fake_generator.rs
│               │   ├── id_generator.rs
│               │   ├── key_generator.rs
│               │   ├── lorem_generator.rs
│               │   ├── sample_file_generator.rs
│               │   └── mod.rs
│           └── main.rs
├── Cargo.toml                  # Workspace manifest
└── README.md
```

The project follows a two-crate architecture:

- **`oxide-dev-tools-core`** — Library containing all implementation logic. Reusable as a dependency in other projects.
- **`oxide-dev-tools-cli`** — Thin CLI layer on top of the core library, using `clap` for argument parsing.

---

## Roadmap

### Phase 1 — Foundation ✅
- [x] CLI scaffold with clap
- [x] UUID generation (v1–v8)
- [x] ULID generation
- [x] NanoID generation
- [x] Workspace architecture (core + CLI)

### Phase 2 — Key & Data Generators
- [x] Password generator (configurable length, character sets)
- [x] Token generator (configurable length, hex/base64, JWT)
- [x] Lorem ipsum generator
- [x] Fake data generator (personas, addresses, companies, etc)
- [x] Sample file generator (PDF, PNG, JPG)

### Phase 3 — Codecs & Converters
- [x] Base64 encode/decode
- [x] Hex encode/decode
- [x] URL encode/decode
- [x] Timestamp converter (Unix ↔ ISO 8601 ↔ human-readable)
- [x] Units converter (data storage, data rate, length, time, mass)
- [x] JSON ↔ YAML ↔ XML conversion

### Phase 4 — Validators
- [ ] Email validator
- [ ] URL/URI validator
- [ ] IP address validator (IPv4, IPv6)
- [ ] UUID validator
- [ ] JSON/YAML syntax validator
- [ ] Credit card number validator (Luhn)
- [ ] Password strength analyzer

### Phase 5 — Text Utilities
- [ ] Case conversion (camelCase, snake_case, kebab-case, etc.)
- [ ] Slugify
- [ ] Word/line/character count
- [ ] Text truncate / ellipsis
- [ ] String encode/decode (HTML entities, unicode escapes)

### Phase 6 — Comparators & Diffs
- [ ] Text diff (line-based)
- [ ] JSON deep compare
- [ ] Semantic version comparison
- [ ] Directory structure comparison

### Phase 7 — File Generators & Scaffolding
- [ ] `.gitignore` generator
- [ ] License file generator
- [ ] Dockerfile generator
- [ ] CI config generator (GitHub Actions, GitLab CI)
- [ ] Boilerplate scaffolding for common project types

### Phase 8 — Structural Analyzers
- [ ] JSON/YAML schema inference
- [ ] Directory tree visualizer
- [ ] Dependency graph analyzer (for Cargo.toml, package.json, etc.)
- [ ] Duplicate file finder

### Phase 9 — Polish & Distribution
- [ ] Shell completions (bash, zsh, fish, PowerShell)
- [ ] Man page generation
- [ ] Pre-built binaries for Linux, macOS, Windows
- [ ] Homebrew formula
- [ ] Scoop / winget packages

---

## Development

```bash
# Run tests
cargo test

# Run tests with nextest (requires cargo-nextest)
cargo nextest run --workspace --all-features --locked

# Run the CLI
cargo run -p oxide-dev-tools-cli -- gen id uuidv4

# Lint
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# Format
cargo fmt --workspace
```

---

## License

This project is licensed under the MIT License. See [LICENSE](./LICENSE) for details.
