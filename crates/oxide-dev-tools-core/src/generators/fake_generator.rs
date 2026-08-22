use std::fmt;

use rand::RngExt;
use rand::rngs::ThreadRng;
use rand::seq::IndexedRandom;

/// Errors that can occur when generating fake data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeError {
    /// A requested count was zero.
    ZeroCount,
}

impl fmt::Display for FakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FakeError::ZeroCount => write!(f, "count must be at least 1"),
        }
    }
}

impl std::error::Error for FakeError {}

/// Kinds of fake data that can be generated.
#[derive(Debug)]
pub enum FakeKind {
    /// A full persona card (name, email, phone, address, company, job title).
    Person(FakeOptions),
    /// A random given name.
    Name(FakeOptions),
    /// A random family name.
    Surname(FakeOptions),
    /// A random full name (given + family).
    FullName(FakeOptions),
    /// A random email address.
    Email(FakeOptions),
    /// A random US phone number.
    Phone(FakeOptions),
    /// A random street address (street, city, country).
    Address(FakeOptions),
    /// A random city.
    City(FakeOptions),
    /// A random country.
    Country(FakeOptions),
    /// A random company name.
    Company(FakeOptions),
    /// A random job title.
    JobTitle(FakeOptions),
    /// A random username (login handle).
    Username(FakeOptions),
}

/// Options shared by every fake data variant.
#[derive(Debug, Clone)]
pub struct FakeOptions {
    /// Number of values to generate.
    pub count: usize,
}

impl Default for FakeOptions {
    fn default() -> Self {
        Self { count: 1 }
    }
}

/// Generate fake data according to `kind`.
///
/// Values are drawn uniformly from fixed curated lists, so repeated calls
/// produce different data while every value stays on-vocabulary.
pub fn generate_fake(kind: FakeKind) -> Result<String, FakeError> {
    match kind {
        FakeKind::Person(opts) => join_counted(opts.count, "\n\n", gen_person),
        FakeKind::Name(opts) => join_counted(opts.count, "\n", |rng| pick(FIRST_NAMES, rng).to_string()),
        FakeKind::Surname(opts) => join_counted(opts.count, "\n", |rng| pick(SURNAMES, rng).to_string()),
        FakeKind::FullName(opts) => {
            join_counted(opts.count, "\n", |rng| format!("{} {}", pick(FIRST_NAMES, rng), pick(SURNAMES, rng)))
        }
        FakeKind::Email(opts) => join_counted(opts.count, "\n", gen_email),
        FakeKind::Phone(opts) => join_counted(opts.count, "\n", gen_phone),
        FakeKind::Address(opts) => join_counted(opts.count, "\n", gen_address),
        FakeKind::City(opts) => join_counted(opts.count, "\n", |rng| pick(CITIES, rng).to_string()),
        FakeKind::Country(opts) => join_counted(opts.count, "\n", |rng| pick(COUNTRIES, rng).to_string()),
        FakeKind::Company(opts) => join_counted(opts.count, "\n", gen_company),
        FakeKind::JobTitle(opts) => join_counted(opts.count, "\n", |rng| pick(JOB_TITLES, rng).to_string()),
        FakeKind::Username(opts) => join_counted(opts.count, "\n", gen_username),
    }
}

// -------- Shared plumbing --------

/// Reject zero counts and join `count` generated values with `sep`.
fn join_counted(
    count: usize,
    sep: &str,
    mut sample: impl FnMut(&mut ThreadRng) -> String,
) -> Result<String, FakeError> {
    if count == 0 {
        return Err(FakeError::ZeroCount);
    }
    let mut rng = rand::rng();
    let values: Vec<String> = (0..count).map(|_| sample(&mut rng)).collect();
    Ok(values.join(sep))
}

/// Sample one entry from a non-empty string list.
fn pick<'a>(items: &'a [&'a str], rng: &mut ThreadRng) -> &'a str {
    items.choose(rng).expect("fake data list must not be empty")
}

// -------- Names --------

/// Common given names.
const FIRST_NAMES: &[&str] = &[
    "James",
    "Mary",
    "Robert",
    "Patricia",
    "John",
    "Jennifer",
    "Michael",
    "Linda",
    "David",
    "Elizabeth",
    "William",
    "Barbara",
    "Richard",
    "Susan",
    "Joseph",
    "Jessica",
    "Thomas",
    "Sarah",
    "Charles",
    "Karen",
    "Christopher",
    "Nancy",
    "Daniel",
    "Lisa",
    "Matthew",
    "Betty",
    "Anthony",
    "Sandra",
    "Mark",
    "Margaret",
    "Donald",
    "Ashley",
    "Steven",
    "Kimberly",
    "Paul",
    "Emily",
    "Andrew",
    "Donna",
    "Joshua",
    "Michelle",
    "Kenneth",
    "Carol",
    "Kevin",
    "Amanda",
    "Brian",
    "Melissa",
    "George",
    "Deborah",
];

/// Common family names.
const SURNAMES: &[&str] = &[
    "Smith",
    "Johnson",
    "Williams",
    "Brown",
    "Jones",
    "Garcia",
    "Miller",
    "Davis",
    "Rodriguez",
    "Martinez",
    "Hernandez",
    "Lopez",
    "Gonzalez",
    "Wilson",
    "Anderson",
    "Thomas",
    "Taylor",
    "Moore",
    "Jackson",
    "Martin",
    "Lee",
    "Perez",
    "Thompson",
    "White",
    "Harris",
    "Sanchez",
    "Clark",
    "Ramirez",
    "Lewis",
    "Robinson",
    "Walker",
    "Young",
    "Allen",
    "King",
    "Wright",
    "Scott",
    "Torres",
    "Nguyen",
    "Hill",
    "Flores",
    "Green",
    "Adams",
    "Nelson",
    "Baker",
    "Hall",
    "Rivera",
    "Campbell",
    "Mitchell",
];

// -------- Addresses --------

/// House numbers range from 1 to `MAX_HOUSE_NUMBER`.
const MAX_HOUSE_NUMBER: u32 = 9_999;

/// Common US street names (single words).
const STREET_NAMES: &[&str] = &[
    "Main",
    "Oak",
    "Maple",
    "Cedar",
    "Pine",
    "Elm",
    "Washington",
    "Lake",
    "Hill",
    "Park",
    "Church",
    "High",
    "Mill",
    "Spring",
    "Walnut",
    "Chestnut",
    "River",
    "Sunset",
    "Union",
    "Center",
    "Bridge",
    "Court",
    "Fairview",
    "Franklin",
    "Grant",
    "Green",
    "Grove",
    "Hamilton",
    "Harrison",
    "Hickory",
    "Jackson",
    "Jefferson",
    "King",
    "Lafayette",
    "Lincoln",
    "Locust",
    "Madison",
    "Meadow",
    "Monroe",
    "Morgan",
];

/// Street suffixes appended to street names.
const STREET_SUFFIXES: &[&str] = &["St", "Ave", "Rd", "Blvd", "Ln", "Dr", "Ct", "Way"];

/// Cities used by the address generator.
const CITIES: &[&str] = &[
    "New York",
    "Los Angeles",
    "Chicago",
    "Houston",
    "Phoenix",
    "Philadelphia",
    "San Antonio",
    "San Diego",
    "Dallas",
    "San Jose",
    "Austin",
    "Jacksonville",
    "Fort Worth",
    "Columbus",
    "Charlotte",
    "Indianapolis",
    "San Francisco",
    "Seattle",
    "Denver",
    "Nashville",
    "Oklahoma City",
    "Portland",
    "Las Vegas",
    "Memphis",
    "Louisville",
    "Baltimore",
    "Milwaukee",
    "Albuquerque",
    "Tucson",
    "Fresno",
    "Sacramento",
    "Kansas City",
];

/// Countries used by the address generator.
const COUNTRIES: &[&str] = &[
    "United States",
    "Canada",
    "Mexico",
    "Brazil",
    "Argentina",
    "United Kingdom",
    "Germany",
    "France",
    "Spain",
    "Italy",
    "Netherlands",
    "Belgium",
    "Switzerland",
    "Austria",
    "Sweden",
    "Norway",
    "Denmark",
    "Finland",
    "Poland",
    "Portugal",
    "Ireland",
    "Australia",
    "New Zealand",
    "Japan",
    "South Korea",
    "China",
    "India",
    "Singapore",
    "Malaysia",
    "South Africa",
    "Egypt",
    "United Arab Emirates",
];

fn gen_address(rng: &mut ThreadRng) -> String {
    let number: u32 = rng.random_range(1..=MAX_HOUSE_NUMBER);
    let street = pick(STREET_NAMES, rng);
    let suffix = pick(STREET_SUFFIXES, rng);
    let city = pick(CITIES, rng);
    let country = pick(COUNTRIES, rng);
    format!("{number} {street} {suffix}, {city}, {country}")
}

// -------- Emails --------

/// Separators joined between the given and family name parts.
const EMAIL_SEPARATORS: &[&str] = &[".", "_", ""];

/// Mail domains. `example.com` is reserved by RFC 2606 for documentation.
const EMAIL_DOMAINS: &[&str] = &[
    "gmail.com",
    "yahoo.com",
    "outlook.com",
    "hotmail.com",
    "protonmail.com",
    "icloud.com",
    "example.com",
    "mail.com",
    "aol.com",
    "zoho.com",
];

fn gen_email(rng: &mut ThreadRng) -> String {
    let first = pick(FIRST_NAMES, rng);
    let last = pick(SURNAMES, rng);
    gen_email_from(first, last, rng)
}

/// Build `first<sep>lastNN@domain` (always lowercased, 2-digit suffix).
fn gen_email_from(first: &str, last: &str, rng: &mut ThreadRng) -> String {
    let separator = pick(EMAIL_SEPARATORS, rng);
    let number: u32 = rng.random_range(0..100);
    let domain = pick(EMAIL_DOMAINS, rng);
    format!("{}{separator}{}{number:02}@{domain}", first.to_lowercase(), last.to_lowercase())
}

// -------- Phones --------

/// US area codes used by the phone generator.
const AREA_CODES: &[&str] = &[
    "202", "212", "213", "301", "305", "310", "312", "313", "317", "404", "407", "408", "410", "412", "415", "502",
    "503", "504", "505", "512", "601", "602", "603", "606", "607", "609", "612", "614", "615", "617",
];

fn gen_phone(rng: &mut ThreadRng) -> String {
    let area = pick(AREA_CODES, rng);
    let exchange: u32 = rng.random_range(200..1000);
    let subscriber: u32 = rng.random_range(0..10_000);
    format!("+1 ({area}) {exchange:03}-{subscriber:04}")
}

// -------- Companies --------

/// Word prefixes combined with a suffix to form company names.
const COMPANY_PREFIXES: &[&str] = &[
    "Acme", "Apex", "Atlas", "Beacon", "Bright", "Cobalt", "Crest", "Diamond", "Eagle", "Ember", "Falcon", "Frost",
    "Global", "Golden", "Harbor", "Iron", "Jade", "Kestrel", "Lunar", "Maple", "Nimbus", "Nova", "Oak", "Orbit",
    "Pinnacle", "Pixel", "Quantum", "Redwood", "Silver", "Summit", "Titan", "Vertex", "Vista", "Willow", "Zenith",
    "Zephyr",
];

/// Word suffixes combined with a prefix to form company names.
const COMPANY_SUFFIXES: &[&str] = &[
    "Industries",
    "Systems",
    "Solutions",
    "Technologies",
    "Labs",
    "Group",
    "Dynamics",
    "Ventures",
    "Partners",
    "Works",
];

fn gen_company(rng: &mut ThreadRng) -> String {
    let prefix = pick(COMPANY_PREFIXES, rng);
    let suffix = pick(COMPANY_SUFFIXES, rng);
    format!("{prefix} {suffix}")
}

// -------- Jobs --------

/// Common job titles.
const JOB_TITLES: &[&str] = &[
    "Software Engineer",
    "Product Manager",
    "Data Scientist",
    "DevOps Engineer",
    "UX Designer",
    "Project Manager",
    "Marketing Specialist",
    "Sales Representative",
    "Account Manager",
    "Financial Analyst",
    "HR Coordinator",
    "Customer Support Lead",
    "Quality Assurance Engineer",
    "Systems Administrator",
    "Network Engineer",
    "Content Writer",
    "Graphic Designer",
    "Business Analyst",
    "Operations Manager",
    "Research Scientist",
    "Technical Writer",
    "Database Administrator",
    "Security Analyst",
    "Solutions Architect",
];

// -------- Usernames --------

/// Separators joined between the given and family name parts.
const USERNAME_SEPARATORS: &[&str] = &[".", "_", ""];

fn gen_username(rng: &mut ThreadRng) -> String {
    let first = pick(FIRST_NAMES, rng).to_lowercase();
    let last = pick(SURNAMES, rng).to_lowercase();
    let separator = pick(USERNAME_SEPARATORS, rng);
    let number: u32 = rng.random_range(0..100);
    format!("{first}{separator}{last}{number:02}")
}

// -------- Person --------

/// A full persona card combining every other fake field.
fn gen_person(rng: &mut ThreadRng) -> String {
    let first = pick(FIRST_NAMES, rng);
    let last = pick(SURNAMES, rng);
    let email = gen_email_from(first, last, rng);
    let phone = gen_phone(rng);
    let address = gen_address(rng);
    let company = gen_company(rng);
    let job = pick(JOB_TITLES, rng);
    format!("Name: {first} {last}\nEmail: {email}\nPhone: {phone}\nAddress: {address}\nCompany: {company}\nJob: {job}")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn opts(count: usize) -> FakeOptions {
        FakeOptions { count }
    }

    fn kinds_with(count: usize) -> Vec<FakeKind> {
        vec![
            FakeKind::Person(opts(count)),
            FakeKind::Name(opts(count)),
            FakeKind::Surname(opts(count)),
            FakeKind::FullName(opts(count)),
            FakeKind::Email(opts(count)),
            FakeKind::Phone(opts(count)),
            FakeKind::Address(opts(count)),
            FakeKind::City(opts(count)),
            FakeKind::Country(opts(count)),
            FakeKind::Company(opts(count)),
            FakeKind::JobTitle(opts(count)),
            FakeKind::Username(opts(count)),
        ]
    }

    /// Assert an email matches `local@domain` with `first<sep>lastNN` local.
    fn assert_email_shape(email: &str) {
        let (local, domain) = email.split_once('@').expect("email must contain @");
        assert!(EMAIL_DOMAINS.contains(&domain), "unexpected domain: {domain}");
        let (name, digits) = local.split_at(local.len() - 2);
        assert!(digits.chars().all(|c| c.is_ascii_digit()), "local must end in 2 digits: {local}");
        let parts: Vec<&str> = name.split(['.', '_']).collect();
        assert!((1..=2).contains(&parts.len()), "unexpected local part: {local}");
        assert!(
            parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase())),
            "local part must be lowercase letters: {local}"
        );
    }

    #[test]
    fn every_kind_returns_non_empty() {
        for kind in kinds_with(1) {
            let value = generate_fake(kind).expect("should generate fake data");
            assert!(!value.is_empty());
        }
    }

    #[test]
    fn zero_count_errors_for_every_kind() {
        for kind in kinds_with(0) {
            let err = generate_fake(kind).unwrap_err();
            assert_eq!(err, FakeError::ZeroCount);
            assert!(err.to_string().contains("at least 1"));
        }
    }

    #[test]
    fn count_joins_values_per_line() {
        for kind in kinds_with(3) {
            let is_person = matches!(&kind, FakeKind::Person(_));
            let text = generate_fake(kind).expect("should generate fake data");
            if is_person {
                assert_eq!(text.split("\n\n").count(), 3, "expected 3 persona cards");
            } else {
                assert_eq!(text.lines().count(), 3, "expected 3 values per line");
            }
        }
    }

    #[test]
    fn person_default_shape() {
        let card = generate_fake(FakeKind::Person(FakeOptions::default())).unwrap();
        let lines: Vec<&str> = card.lines().collect();
        assert_eq!(lines.len(), 6);
        assert!(lines[0].starts_with("Name: "));
        assert!(lines[1].starts_with("Email: "));
        assert!(lines[2].starts_with("Phone: "));
        assert!(lines[3].starts_with("Address: "));
        assert!(lines[4].starts_with("Company: "));
        assert!(lines[5].starts_with("Job: "));
    }

    #[test]
    fn person_name_uses_known_lists() {
        for _ in 0..100 {
            let card = generate_fake(FakeKind::Person(FakeOptions::default())).unwrap();
            let name_line = card.lines().next().expect("person card has a name line");
            let full = name_line.strip_prefix("Name: ").expect("name line is prefixed");
            let mut parts = full.split_whitespace();
            let first = parts.next().expect("full name has a given name");
            let last = parts.next().expect("full name has a family name");
            assert!(FIRST_NAMES.contains(&first), "unexpected given name: {first}");
            assert!(SURNAMES.contains(&last), "unexpected family name: {last}");
        }
    }

    #[test]
    fn name_from_known_list() {
        for _ in 0..100 {
            let name = generate_fake(FakeKind::Name(FakeOptions::default())).unwrap();
            assert!(FIRST_NAMES.contains(&name.as_str()), "unexpected name: {name}");
        }
    }

    #[test]
    fn name_variety_across_1000_samples() {
        let mut seen = HashSet::new();
        for _ in 0..1_000 {
            let name = generate_fake(FakeKind::Name(FakeOptions::default())).unwrap();
            seen.insert(name);
        }
        assert!(seen.len() >= 30, "only {} distinct names across 1,000 samples", seen.len());
    }

    #[test]
    fn surname_from_known_list() {
        for _ in 0..100 {
            let surname = generate_fake(FakeKind::Surname(FakeOptions::default())).unwrap();
            assert!(SURNAMES.contains(&surname.as_str()), "unexpected surname: {surname}");
        }
    }

    #[test]
    fn fullname_combines_known_lists() {
        let full = generate_fake(FakeKind::FullName(FakeOptions::default())).unwrap();
        let parts: Vec<&str> = full.split_whitespace().collect();
        assert_eq!(parts.len(), 2, "unexpected full name: {full}");
        assert!(FIRST_NAMES.contains(&parts[0]), "unexpected given name: {}", parts[0]);
        assert!(SURNAMES.contains(&parts[1]), "unexpected family name: {}", parts[1]);
    }

    #[test]
    fn email_shape() {
        for _ in 0..100 {
            let email = generate_fake(FakeKind::Email(FakeOptions::default())).unwrap();
            assert_email_shape(&email);
        }
    }

    #[test]
    fn email_variety_across_1000_samples() {
        let mut seen = HashSet::new();
        for _ in 0..1_000 {
            let email = generate_fake(FakeKind::Email(FakeOptions::default())).unwrap();
            seen.insert(email);
        }
        assert!(seen.len() >= 900, "only {} distinct emails across 1,000 samples", seen.len());
    }

    #[test]
    fn phone_shape() {
        for _ in 0..100 {
            let phone = generate_fake(FakeKind::Phone(FakeOptions::default())).unwrap();
            assert_eq!(phone.len(), 17, "unexpected phone: {phone}");
            assert!(phone.starts_with("+1 ("), "unexpected phone: {phone}");
            let area = &phone[4..7];
            assert!(AREA_CODES.contains(&area), "unexpected area code: {area}");
            assert_eq!(&phone[7..9], ") ", "unexpected phone: {phone}");
            assert_eq!(&phone[12..13], "-", "unexpected phone: {phone}");
            let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
            assert_eq!(digits.len(), 11, "unexpected phone: {phone}");
        }
    }

    #[test]
    fn phone_variety_across_1000_samples() {
        let mut seen = HashSet::new();
        for _ in 0..1_000 {
            let phone = generate_fake(FakeKind::Phone(FakeOptions::default())).unwrap();
            seen.insert(phone);
        }
        assert!(seen.len() >= 990, "only {} distinct phones across 1,000 samples", seen.len());
    }

    #[test]
    fn address_shape() {
        for _ in 0..100 {
            let address = generate_fake(FakeKind::Address(FakeOptions::default())).unwrap();
            let parts: Vec<&str> = address.split(", ").collect();
            assert_eq!(parts.len(), 3, "unexpected address: {address}");
            let street_parts: Vec<&str> = parts[0].split_whitespace().collect();
            assert_eq!(street_parts.len(), 3, "unexpected address: {address}");
            assert!(street_parts[0].parse::<u32>().is_ok(), "unexpected house number: {}", parts[0]);
            assert!(STREET_NAMES.contains(&street_parts[1]), "unexpected street: {}", parts[0]);
            assert!(STREET_SUFFIXES.contains(&street_parts[2]), "unexpected suffix: {}", parts[0]);
            assert!(CITIES.contains(&parts[1]), "unexpected city: {}", parts[1]);
            assert!(COUNTRIES.contains(&parts[2]), "unexpected country: {}", parts[2]);
        }
    }

    #[test]
    fn city_from_known_list() {
        for _ in 0..100 {
            let city = generate_fake(FakeKind::City(FakeOptions::default())).unwrap();
            assert!(CITIES.contains(&city.as_str()), "unexpected city: {city}");
        }
    }

    #[test]
    fn city_variety_across_1000_samples() {
        let mut seen = HashSet::new();
        for _ in 0..1_000 {
            let city = generate_fake(FakeKind::City(FakeOptions::default())).unwrap();
            seen.insert(city);
        }
        assert!(seen.len() >= 25, "only {} distinct cities across 1,000 samples", seen.len());
    }

    #[test]
    fn country_from_known_list() {
        for _ in 0..100 {
            let country = generate_fake(FakeKind::Country(FakeOptions::default())).unwrap();
            assert!(COUNTRIES.contains(&country.as_str()), "unexpected country: {country}");
        }
    }

    #[test]
    fn company_shape() {
        for _ in 0..100 {
            let company = generate_fake(FakeKind::Company(FakeOptions::default())).unwrap();
            let parts: Vec<&str> = company.split_whitespace().collect();
            assert_eq!(parts.len(), 2, "unexpected company: {company}");
            assert!(COMPANY_PREFIXES.contains(&parts[0]), "unexpected prefix: {}", parts[0]);
            assert!(COMPANY_SUFFIXES.contains(&parts[1]), "unexpected suffix: {}", parts[1]);
        }
    }

    #[test]
    fn job_title_from_known_list() {
        for _ in 0..100 {
            let job = generate_fake(FakeKind::JobTitle(FakeOptions::default())).unwrap();
            assert!(JOB_TITLES.contains(&job.as_str()), "unexpected job title: {job}");
        }
    }

    #[test]
    fn username_shape() {
        for _ in 0..100 {
            let username = generate_fake(FakeKind::Username(FakeOptions::default())).unwrap();
            assert!(username.len() >= 6, "unexpected username: {username}");
            assert!(
                username
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'),
                "unexpected username: {username}"
            );
            let (name, digits) = username.split_at(username.len() - 2);
            assert!(digits.chars().all(|c| c.is_ascii_digit()), "username must end in 2 digits: {username}");
            let parts: Vec<&str> = name.split(['.', '_']).collect();
            assert!(
                parts
                    .iter()
                    .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase())),
                "username parts must be lowercase letters: {username}"
            );
        }
    }
}
