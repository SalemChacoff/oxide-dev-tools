use clap::{Args, Subcommand};
use oxide_dev_tools_core::*;

use crate::error::GenError;

/// `oxide gen fake [subcommand]` — fake data generator dispatch
#[derive(Args)]
pub struct FakeArgs {
    #[command(subcommand)]
    pub kind: FakeCmd,
}

#[derive(Subcommand)]
pub enum FakeCmd {
    /// Generate a full persona (name, email, phone, address, company, job title)
    #[command(
        name = "person",
        after_help = "Examples:\n  oxide gen fake person\n  oxide gen fake person --count 3"
    )]
    Person {
        /// Number of personas to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random given name
    #[command(
        name = "name",
        after_help = "Examples:\n  oxide gen fake name\n  oxide gen fake name --count 5"
    )]
    Name {
        /// Number of names to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random family name
    #[command(
        name = "surname",
        after_help = "Examples:\n  oxide gen fake surname\n  oxide gen fake surname --count 5"
    )]
    Surname {
        /// Number of surnames to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random full name (given + family)
    #[command(
        name = "fullname",
        after_help = "Examples:\n  oxide gen fake fullname\n  oxide gen fake fullname --count 5"
    )]
    FullName {
        /// Number of full names to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random email address
    #[command(
        name = "email",
        after_help = "Examples:\n  oxide gen fake email\n  oxide gen fake email --count 10"
    )]
    Email {
        /// Number of emails to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random US phone number
    #[command(
        name = "phone",
        after_help = "Examples:\n  oxide gen fake phone\n  oxide gen fake phone --count 3"
    )]
    Phone {
        /// Number of phone numbers to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random street address (street, city, country)
    #[command(
        name = "address",
        after_help = "Examples:\n  oxide gen fake address\n  oxide gen fake address --count 3"
    )]
    Address {
        /// Number of addresses to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random city
    #[command(
        name = "city",
        after_help = "Examples:\n  oxide gen fake city\n  oxide gen fake city --count 5"
    )]
    City {
        /// Number of cities to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random country
    #[command(
        name = "country",
        after_help = "Examples:\n  oxide gen fake country\n  oxide gen fake country --count 5"
    )]
    Country {
        /// Number of countries to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random company name
    #[command(
        name = "company",
        after_help = "Examples:\n  oxide gen fake company\n  oxide gen fake company --count 5"
    )]
    Company {
        /// Number of company names to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random job title
    #[command(
        name = "job",
        after_help = "Examples:\n  oxide gen fake job\n  oxide gen fake job --count 5"
    )]
    Job {
        /// Number of job titles to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random username (login handle)
    #[command(
        name = "username",
        after_help = "Examples:\n  oxide gen fake username\n  oxide gen fake username --count 5"
    )]
    Username {
        /// Number of usernames to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },
}

pub fn exec(args: FakeArgs) -> Result<(), GenError> {
    let kind = match args.kind {
        FakeCmd::Person { count } => FakeKind::Person(FakeOptions { count }),
        FakeCmd::Name { count } => FakeKind::Name(FakeOptions { count }),
        FakeCmd::Surname { count } => FakeKind::Surname(FakeOptions { count }),
        FakeCmd::FullName { count } => FakeKind::FullName(FakeOptions { count }),
        FakeCmd::Email { count } => FakeKind::Email(FakeOptions { count }),
        FakeCmd::Phone { count } => FakeKind::Phone(FakeOptions { count }),
        FakeCmd::Address { count } => FakeKind::Address(FakeOptions { count }),
        FakeCmd::City { count } => FakeKind::City(FakeOptions { count }),
        FakeCmd::Country { count } => FakeKind::Country(FakeOptions { count }),
        FakeCmd::Company { count } => FakeKind::Company(FakeOptions { count }),
        FakeCmd::Job { count } => FakeKind::JobTitle(FakeOptions { count }),
        FakeCmd::Username { count } => FakeKind::Username(FakeOptions { count }),
    };
    println!("{}", generate_fake(kind)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_person_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Person { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_person_multiple() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Person { count: 3 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_name_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Name { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_surname_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Surname { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_fullname_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::FullName { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_email_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Email { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_phone_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Phone { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_address_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Address { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_city_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::City { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_country_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Country { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_company_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Company { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_job_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Job { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_username_default() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Username { count: 1 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_multiple_values() {
        assert!(
            exec(FakeArgs {
                kind: FakeCmd::Email { count: 5 }
            })
            .is_ok()
        );
    }

    #[test]
    fn exec_zero_count_errors() {
        let result = exec(FakeArgs {
            kind: FakeCmd::Email { count: 0 },
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least 1"));
    }
}
