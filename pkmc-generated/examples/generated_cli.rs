use std::{error::Error, path::PathBuf};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Download specified version.
    Download {
        #[arg(short, long)]
        /// Minecraft version ID to download (e.g. "1.21.4")
        version: String,
        #[arg(short, long)]
        /// Output file path (e.g. "assets/generated/server.jar")
        output: PathBuf,
    },
    /// Extract minecraft .jar generated data
    Extract {
        #[arg(short, long)]
        /// Input minecraft .jar
        input: PathBuf,
        #[arg(short, long)]
        /// Output directory to extract generated data to
        output: PathBuf,
        #[arg(short, long, default_value_t = false)]
        /// Delete everything in output directory before converting
        clean: bool,
    },
    /// Generate source code from generated data
    Source {
        #[arg(short, long)]
        /// Input generated data directory
        input: PathBuf,
        #[arg(short, long)]
        /// Output file for generated code
        output: PathBuf,
        #[arg(short, long, default_value_t = false)]
        /// If to skip using rustfmt on generated code
        skip_format: bool,
    },
}

// TODO: clean args
#[allow(unused)]
fn main() -> Result<(), Box<dyn Error>> {
    match Args::parse().command {
        Commands::Download { version, output } => {
            pkmc_generated::download_server_jar(&version, output)?;
        }
        Commands::Extract {
            input,
            output,
            clean,
        } => {
            pkmc_generated::extract_generated_data(input, output, true)?;
        }
        Commands::Source {
            input,
            output,
            skip_format,
        } => {
            pkmc_generated::generate_generated_code(input, output, skip_format)?;
        }
    }
    Ok(())
}
