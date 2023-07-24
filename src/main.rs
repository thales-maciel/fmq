use fmq::prelude::*;

use clap::{Parser, Subcommand};

fn main() -> Result<()> {
    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
}

