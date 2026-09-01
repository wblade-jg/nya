use clap::{Parser, Subcommand};
use nya::commands::update::update;
use nya::configuration::Configuration;

#[derive(Parser)]
#[command(name = "nya", about = "a minimal package manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    ///Updates the list of repositories
    Update,
    ///Installs a package
    Install { 
        ///Name of the package
        package_name: String 
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Configuration::from_env();

    let cli = Cli::parse();

    match cli.command {
        Commands::Update => {
            update(&config).await;
        },
        Commands::Install { package_name } => {
            println!("Instalando: {}", package_name);
        }
    }

    Ok(())
}
