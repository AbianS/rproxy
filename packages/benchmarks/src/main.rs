//! rproxy-bench: Benchmark suite for comparing reverse proxies
//!
//! Compares rproxy against nginx, traefik, caddy, and envoy
//! under identical conditions in Docker.

mod config;
mod metrics;
mod report;
mod runner;
mod workload;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "rproxy-bench")]
#[command(about = "Benchmark suite for rproxy - comparing reverse proxies")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info", global = true)]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a benchmark scenario
    Run {
        /// Scenario name or path to TOML file
        #[arg(short, long)]
        scenario: String,

        /// Target proxy to benchmark (rproxy, nginx, traefik, caddy, envoy, all)
        #[arg(short, long, default_value = "all")]
        target: String,

        /// Output directory for results
        #[arg(short, long, default_value = "results")]
        output: PathBuf,

        /// Skip warmup phase
        #[arg(long)]
        no_warmup: bool,

        /// Custom upstream URL (overrides config)
        #[arg(long)]
        upstream: Option<String>,
    },

    /// List available scenarios
    List,

    /// Compare benchmark results
    Compare {
        /// First result file (baseline)
        baseline: PathBuf,

        /// Second result file (comparison)
        comparison: PathBuf,
    },

    /// Show results from a benchmark run
    Show {
        /// Result file to display
        result: PathBuf,
    },

    /// Start all proxy containers
    Up {
        /// Detach and run in background
        #[arg(short, long)]
        detach: bool,
    },

    /// Stop all proxy containers
    Down,

    /// Show container resource usage
    Stats,

    /// Generate comparison report (markdown/html)
    Report {
        /// Results directory
        #[arg(short, long, default_value = "results")]
        input: PathBuf,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,

        /// Output format (markdown, html, json)
        #[arg(short, long, default_value = "markdown")]
        format: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(&cli.log_level)
        }))
        .init();

    match cli.command {
        Commands::Run {
            scenario,
            target,
            output,
            no_warmup,
            upstream,
        } => {
            run_benchmark(&scenario, &target, &output, no_warmup, upstream).await?;
        }

        Commands::List => {
            list_scenarios();
        }

        Commands::Compare {
            baseline,
            comparison,
        } => {
            report::compare_results(&baseline, &comparison)?;
        }

        Commands::Show { result } => {
            report::show_result(&result)?;
        }

        Commands::Up { detach } => {
            docker_up(detach).await?;
        }

        Commands::Down => {
            docker_down().await?;
        }

        Commands::Stats => {
            show_stats().await?;
        }

        Commands::Report {
            input,
            output,
            format,
        } => {
            report::generate_report(&input, &output, &format)?;
        }
    }

    Ok(())
}

async fn run_benchmark(
    scenario: &str,
    target: &str,
    output: &PathBuf,
    no_warmup: bool,
    upstream: Option<String>,
) -> Result<()> {
    println!(
        "\n{} {} benchmark\n",
        "Starting".green().bold(),
        scenario.cyan()
    );

    // Load scenario config
    let config = if scenario.ends_with(".toml") {
        config::ScenarioConfig::from_file(scenario)?
    } else {
        config::ScenarioConfig::preset(scenario)?
    };

    // Determine which proxies to benchmark
    let targets = match target {
        "all" => vec!["rproxy", "nginx", "traefik", "caddy", "envoy"],
        t => vec![t],
    };

    // Create output directory
    std::fs::create_dir_all(output)?;

    // Run benchmarks for each target
    let mut all_results = Vec::new();

    for proxy in targets {
        println!("\n{} {}\n", "Benchmarking:".yellow().bold(), proxy.cyan());

        let result = runner::run_scenario(&config, proxy, !no_warmup, upstream.as_deref()).await?;

        // Save individual result
        let filename = format!(
            "{}-{}-{}.json",
            chrono::Utc::now().format("%Y%m%d-%H%M%S"),
            scenario,
            proxy
        );
        let path = output.join(&filename);
        result.save(&path)?;
        println!("  {} {}", "Saved:".green(), path.display());

        all_results.push((proxy.to_string(), result));
    }

    // Print comparison table if multiple proxies
    if all_results.len() > 1 {
        println!("\n{}\n", "Comparison Summary".yellow().bold());
        report::print_comparison_table(&all_results);
    }

    Ok(())
}

fn list_scenarios() {
    println!("\n{}\n", "Available Scenarios".yellow().bold());

    let scenarios = [
        ("baseline", "Minimal load (1 req/s) - measures base latency"),
        ("sustained", "Sustained load (1k req/s for 5 min) - stability test"),
        ("burst", "Traffic spikes (10k requests, pause, repeat) - spike handling"),
        ("stress", "Ramp-up until 5% errors - find breaking point"),
        ("memory", "Long-running (30 min) - memory leak detection"),
        ("concurrent", "High concurrency (1k connections) - connection handling"),
    ];

    for (name, desc) in scenarios {
        println!("  {} - {}", name.cyan(), desc);
    }

    println!("\n{}", "Use a custom TOML file for advanced scenarios.".dimmed());
}

async fn docker_up(detach: bool) -> Result<()> {
    println!("{}", "Starting proxy containers...".yellow());

    let status = if detach {
        std::process::Command::new("docker")
            .args(["compose", "-f", "docker-compose.yml", "up", "-d"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()?
    } else {
        std::process::Command::new("docker")
            .args(["compose", "-f", "docker-compose.yml", "up"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()?
    };

    if status.success() {
        println!("{}", "Containers started successfully.".green());
    } else {
        anyhow::bail!("Failed to start containers");
    }

    Ok(())
}

async fn docker_down() -> Result<()> {
    println!("{}", "Stopping proxy containers...".yellow());

    let status = std::process::Command::new("docker")
        .args(["compose", "-f", "docker-compose.yml", "down"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()?;

    if status.success() {
        println!("{}", "Containers stopped.".green());
    }

    Ok(())
}

async fn show_stats() -> Result<()> {
    let docker = bollard::Docker::connect_with_local_defaults()?;

    let containers = [
        "bench-rproxy",
        "bench-nginx",
        "bench-traefik",
        "bench-caddy",
        "bench-envoy",
        "bench-upstream",
    ];

    println!("\n{}\n", "Container Resource Usage".yellow().bold());

    for container in containers {
        match metrics::get_container_stats(&docker, container).await {
            Ok(stats) => {
                println!(
                    "  {} CPU: {:.1}% | Memory: {:.1} MB",
                    format!("{:15}", container).cyan(),
                    stats.cpu_percent,
                    stats.memory_mb
                );
            }
            Err(_) => {
                println!("  {} {}", format!("{:15}", container).dimmed(), "not running".dimmed());
            }
        }
    }

    Ok(())
}
