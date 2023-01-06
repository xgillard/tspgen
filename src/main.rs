use clap::{Parser, Subcommand};
use generation::GenerateInstance;
use resolution::Solve;
use visualisation::Visualize;


mod instance;
mod generation;
mod visualisation;
mod resolution;

/// TspGen is a generator for realistic TSP instances where the cities to visit are gouped in clusters.
/// 
/// Generate instance in Belgium:
/// ```
/// ./target/release/tspgen  --min-longitude=2.376776  --max-longitude=5.91469  --min-latitude=50.2840167  --max-latitude=51.034368
/// ```
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct TspTools {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Generate(GenerateInstance),
    Visualize(Visualize),
    Solve(Solve)
}

#[tokio::main]
async fn main() {
    let cli = TspTools::parse();
    match cli.command {
        Command::Generate(generate) => generate.execute().await,
        Command::Visualize(visualize) => visualize.execute().await,
        Command::Solve(solve) => solve.execute().await
    }
}