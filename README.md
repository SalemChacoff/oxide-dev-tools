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

### 🚧 Planned / In progress

| Category | Description |
|---|---|
| **Validators** | Validate emails, URLs, IPs, UUIDs, JSON, YAML, credit cards, and more |
| **Comparators** | Diff text, JSON, directories; semantic version compare |
| **Text Utilities** | Case conversion, slugify, count (words/lines/chars), truncate, encode/decode |
| **Codecs** | Base64, hex, URL encode, PEM/PFX parsing, ZIP compression |
| **Converters** | Timestamp ↔ date, units, JSON ↔ YAML, color formats |
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

# Show help
oxide --help
oxide gen --help
oxide gen id --help
oxide gen lorem --help
oxide gen fake --help
```

---

## Project Structure

```
oxide-dev-tools/
├── crates/
│   ├── oxide-dev-tools-core/   # Core library — all logic lives here
│   │   └── src/
│   │       ├── generators/
│   │       │   ├── fake_generator.rs  # Fake personas, names, emails, phones, addresses
│   │       │   ├── id_generator.rs   # UUID v1–v8, ULID, NanoID + planned IDs
│   │       │   ├── jwt_generator.rs  # JWT (HS256) generation
│   │       │   ├── key_generator.rs  # Password, token generation
│   │       │   ├── lorem_generator.rs # Lorem ipsum words, sentences, paragraphs
│   │       │   └── mod.rs
│   │       └── lib.rs
│   └── oxide-dev-tools-cli/    # CLI binary — clap-based argument parsing
│       └── src/
│   │           ├── generators/     # CLI subcommand wrappers for generators
│   │           │   ├── fake_generator.rs
│   │           │   ├── id_generator.rs
│           │   ├── key_generator.rs
│           │   ├── lorem_generator.rs
│           │   └── mod.rs
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
- [ ] Sample file generator (CSV, JSON, YAML)

### Phase 3 — Codecs & Converters
- [ ] Base64 encode/decode
- [ ] Hex encode/decode
- [ ] URL encode/decode
- [ ] Timestamp converter (Unix ↔ ISO 8601 ↔ human-readable)
- [ ] Units converter (bytes, time, etc.)
- [ ] JSON ↔ YAML conversion

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
