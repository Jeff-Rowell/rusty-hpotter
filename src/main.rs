use anyhow;
use bollard::Docker;
use clap::Parser;
use hpotter::{config, docker};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = config::load_config(&args.config)?;
    let docker = Arc::new(docker::connect()?);
    run(&config, docker).await
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the YAML configuration file
    #[arg(short, long, default_value_t = String::from("config.yml"))]
    config: String,
}

async fn run(config: &config::Config, docker_client: Arc<Docker>) -> anyhow::Result<()> {
    docker::download_images(config, docker_client).await?;
    Ok(())
    // TODO: write the database related things:
    // - ensure database exists
    // - ensure tables exists
    // - write functionality for each of the tables
    // - a connection/abstraction that can be passed around to concurrently write to the db from
    // multiple threads
    //
    // for each service in the config, start a thread pool listener for that service
    //
    // for each listener, when a connection is received, start a container thread
    //
    // for each container thread, start the container and attach two one way threads to it
    //
    // when the thread times out or is exited, parse the container logs using the patterns
    //
    // store the parsed data in the database
}
