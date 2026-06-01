use clap::{Parser, Subcommand};
use oxide_dev_tools_core::*;

// TODO: Refactor to odev command or better invoker string, and refactor to scalable CLI tool like
// odev gen id --uuid-v1 argument
// odev gen data --name 10
// etc.
#[derive(Parser)]
#[command(name = "oxide", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Id {
        #[command(subcommand)]
        format: IdFormat,
    },
}

#[derive(Subcommand)]
enum IdFormat {
    /// UUID v1 (timestamp + MAC)
    UuidV1 {
        /// Unix timestamp in seconds (default: now)
        #[arg(short, long)]
        timestamp: Option<u64>,
    },
    /// UUID v3 (MD5 namespace)
    UuidV3,
    /// UUID v4 (random)
    UuidV4,
    /// UUID v5 (SHA-1 namespace)
    UuidV5,
    /// UUID v6 (reordered timestamp + MAC)
    UuidV6 {
        /// Unix timestamp in seconds (default: now)
        #[arg(short, long)]
        timestamp: Option<u64>,
    },
    /// UUID v7 (unix timestamp)
    UuidV7 {
        /// Unix timestamp in seconds (default: now)
        #[arg(short, long)]
        timestamp: Option<u64>,
    },
    /// UUID v8 (custom)
    UuidV8,
    /// ULID
    Ulid,
    /// NanoID
    NanoId,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Id { format } => match format {
            IdFormat::UuidV1 { timestamp } => {
                let t = timestamp.map(unix_secs_to_system_time);
                println!("{}", gen_uuid_v1(t));
            }
            IdFormat::UuidV3 => println!("{}", gen_uuid_v3()),
            IdFormat::UuidV4 => println!("{}", gen_uuid_v4()),
            IdFormat::UuidV5 => println!("{}", gen_uuid_v5()),
            IdFormat::UuidV6 { timestamp } => {
                let t = timestamp.map(unix_secs_to_system_time);
                println!("{}", gen_uuid_v6(t));
            }
            IdFormat::UuidV7 { timestamp } => {
                let t = timestamp.map(unix_secs_to_system_time);
                println!("{}", gen_uuid_v7(t));
            }
            IdFormat::UuidV8 => println!("{}", gen_uuid_v8()),
            IdFormat::Ulid => println!("{}", gen_ulid()),
            IdFormat::NanoId => println!("{}", gen_nanoid()),
        },
    }
}

fn unix_secs_to_system_time(secs: u64) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs)
}
