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
    #[command(name = "person")]
    Person {
        /// Number of personas to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random given name
    #[command(name = "name")]
    Name {
        /// Number of names to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random family name
    #[command(name = "surname")]
    Surname {
        /// Number of surnames to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random full name (given + family)
    #[command(name = "fullname")]
    FullName {
        /// Number of full names to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random email address
    #[command(name = "email")]
    Email {
        /// Number of emails to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random US phone number
    #[command(name = "phone")]
    Phone {
        /// Number of phone numbers to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random street address (street, city, country)
    #[command(name = "address")]
    Address {
        /// Number of addresses to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random city
    #[command(name = "city")]
    City {
        /// Number of cities to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random country
    #[command(name = "country")]
    Country {
        /// Number of countries to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random company name
    #[command(name = "company")]
    Company {
        /// Number of company names to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random job title
    #[command(name = "job")]
    Job {
        /// Number of job titles to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },

    /// Generate a random username (login handle)
    #[command(name = "username")]
    Username {
        /// Number of usernames to generate
        #[arg(short = 'c', long = "count", default_value_t = 1)]
        count: usize,
    },
}

pub fn exec(args: FakeArgs) -> Result<(), GenError> {
    match args.kind {
        FakeCmd::Person { count } => println!("{}", generate_fake(FakeKind::Person(FakeOptions { count }))?),
        FakeCmd::Name { count } => println!("{}", generate_fake(FakeKind::Name(FakeOptions { count }))?),
        FakeCmd::Surname { count } => println!("{}", generate_fake(FakeKind::Surname(FakeOptions { count }))?),
        FakeCmd::FullName { count } => println!("{}", generate_fake(FakeKind::FullName(FakeOptions { count }))?),
        FakeCmd::Email { count } => println!("{}", generate_fake(FakeKind::Email(FakeOptions { count }))?),
        FakeCmd::Phone { count } => println!("{}", generate_fake(FakeKind::Phone(FakeOptions { count }))?),
        FakeCmd::Address { count } => println!("{}", generate_fake(FakeKind::Address(FakeOptions { count }))?),
        FakeCmd::City { count } => println!("{}", generate_fake(FakeKind::City(FakeOptions { count }))?),
        FakeCmd::Country { count } => println!("{}", generate_fake(FakeKind::Country(FakeOptions { count }))?),
        FakeCmd::Company { count } => println!("{}", generate_fake(FakeKind::Company(FakeOptions { count }))?),
        FakeCmd::Job { count } => println!("{}", generate_fake(FakeKind::JobTitle(FakeOptions { count }))?),
        FakeCmd::Username { count } => println!("{}", generate_fake(FakeKind::Username(FakeOptions { count }))?),
    }
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
