//! Command-line interface for arkai.
//!
//! Provides commands for running pipelines, checking status,
//! listing runs, and resuming failed runs.

use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use uuid::Uuid;

use crate::core::{Orchestrator, Pipeline};

/// arkai - Event-sourced AI pipeline orchestrator
#[derive(Parser, Debug)]
#[command(name = "arkai")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a pipeline
    Run {
        /// Pipeline name (will look for pipelines/<name>.yaml)
        pipeline_name: String,

        /// Input file (reads from stdin if not provided)
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Read input from stdin
        #[arg(long)]
        stdin: bool,
    },

    /// Check the status of a run
    Status {
        /// Run ID (UUID)
        run_id: String,
    },

    /// List recent runs
    Runs {
        /// Maximum number of runs to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Resume a failed run
    Resume {
        /// Run ID to resume
        run_id: String,
    },

    /// Start as HTTP server (stub - not yet implemented)
    Serve {
        /// Address to bind to
        #[arg(short, long, default_value = ":9000")]
        address: String,
    },
}

impl Cli {
    /// Execute the CLI command
    pub async fn execute(self) -> Result<()> {
        match self.command {
            Commands::Run {
                pipeline_name,
                input,
                stdin,
            } => {
                run_pipeline(&pipeline_name, input, stdin).await
            }
            Commands::Status { run_id } => {
                show_status(&run_id).await
            }
            Commands::Runs { limit } => {
                list_runs(limit).await
            }
            Commands::Resume { run_id } => {
                resume_run(&run_id).await
            }
            Commands::Serve { address } => {
                serve(&address).await
            }
        }
    }
}

/// Run a pipeline with the given input
async fn run_pipeline(
    pipeline_name: &str,
    input_file: Option<PathBuf>,
    use_stdin: bool,
) -> Result<()> {
    // Load the pipeline
    let pipeline = load_pipeline(pipeline_name)?;

    // Get input
    let input = if let Some(path) = input_file {
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read input file: {}", path.display()))?
    } else if use_stdin || atty::isnt(atty::Stream::Stdin) {
        // Read from stdin if --stdin flag or if stdin is piped
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        buffer
    } else {
        anyhow::bail!("No input provided. Use --input <file> or pipe to stdin");
    };

    if input.trim().is_empty() {
        anyhow::bail!("Input is empty");
    }

    // Execute the pipeline
    let orchestrator = Orchestrator::new();
    let run = orchestrator.run_pipeline(&pipeline, input).await?;

    // Print results
    match &run.state {
        crate::domain::RunState::Completed => {
            // Print the final output
            if let Some(last_step) = pipeline.steps.last() {
                if let Some(artifact) = run.artifacts.get(&last_step.name) {
                    println!("{}", artifact.content);
                }
            }
            eprintln!("\n[Run {} completed successfully]", run.id);
        }
        crate::domain::RunState::Failed { error } => {
            eprintln!("\n[Run {} failed: {}]", run.id, error);
            std::process::exit(1);
        }
        crate::domain::RunState::SafetyLimitReached { limit } => {
            eprintln!("\n[Run {} stopped: safety limit reached - {}]", run.id, limit);
            std::process::exit(1);
        }
        _ => {
            eprintln!("\n[Run {} in state: {:?}]", run.id, run.state);
        }
    }

    Ok(())
}

/// Show the status of a run
async fn show_status(run_id_str: &str) -> Result<()> {
    let run_id = Uuid::parse_str(run_id_str)
        .with_context(|| format!("Invalid run ID: {}", run_id_str))?;

    let orchestrator = Orchestrator::new();
    let run = orchestrator.get_run_status(run_id).await?;

    println!("Run ID: {}", run.id);
    println!("Pipeline: {}", run.pipeline_name);
    println!("State: {:?}", run.state);
    println!("Started: {}", run.started_at);
    if let Some(completed) = run.completed_at {
        println!("Completed: {}", completed);
    }
    println!("Current step: {}", run.current_step);
    println!("\nStep statuses:");
    for (step, status) in &run.step_statuses {
        println!("  {}: {:?}", step, status);
    }

    Ok(())
}

/// List recent runs
async fn list_runs(limit: usize) -> Result<()> {
    let orchestrator = Orchestrator::new();
    let runs = orchestrator.list_runs(limit).await?;

    if runs.is_empty() {
        println!("No runs found");
        return Ok(());
    }

    println!("{:<38} {:<20} {:<15}", "RUN ID", "PIPELINE", "STATE");
    println!("{}", "-".repeat(75));

    for run in runs {
        let state_str = match &run.state {
            crate::domain::RunState::Running => "running".to_string(),
            crate::domain::RunState::Completed => "completed".to_string(),
            crate::domain::RunState::Failed { .. } => "failed".to_string(),
            crate::domain::RunState::Paused => "paused".to_string(),
            crate::domain::RunState::SafetyLimitReached { .. } => "safety-limit".to_string(),
        };
        println!("{:<38} {:<20} {:<15}", run.id, run.pipeline_name, state_str);
    }

    Ok(())
}

/// Resume a failed run
async fn resume_run(run_id_str: &str) -> Result<()> {
    let run_id = Uuid::parse_str(run_id_str)
        .with_context(|| format!("Invalid run ID: {}", run_id_str))?;

    // First get the run to find out which pipeline and input
    let orchestrator = Orchestrator::new();
    let existing_run = orchestrator.get_run_status(run_id).await?;

    // Load the pipeline
    let pipeline = load_pipeline(&existing_run.pipeline_name)?;

    // Resume with original input
    let run = orchestrator
        .resume_run(run_id, &pipeline, existing_run.input)
        .await?;

    // Print results
    match &run.state {
        crate::domain::RunState::Completed => {
            if let Some(last_step) = pipeline.steps.last() {
                if let Some(artifact) = run.artifacts.get(&last_step.name) {
                    println!("{}", artifact.content);
                }
            }
            eprintln!("\n[Run {} resumed and completed successfully]", run.id);
        }
        crate::domain::RunState::Failed { error } => {
            eprintln!("\n[Run {} failed again: {}]", run.id, error);
            std::process::exit(1);
        }
        _ => {
            eprintln!("\n[Run {} in state: {:?}]", run.id, run.state);
        }
    }

    Ok(())
}

/// Start HTTP server (stub)
async fn serve(address: &str) -> Result<()> {
    anyhow::bail!(
        "HTTP server mode not yet implemented. Would serve on {}",
        address
    )
}

/// Load a pipeline by name
fn load_pipeline(name: &str) -> Result<Pipeline> {
    // Look in pipelines/ directory
    let pipeline_path = PathBuf::from("pipelines").join(format!("{}.yaml", name));

    if !pipeline_path.exists() {
        // Try looking in the current directory
        let alt_path = PathBuf::from(format!("{}.yaml", name));
        if alt_path.exists() {
            let pipeline = Pipeline::from_file(&alt_path)?;
            pipeline.validate()?;
            return Ok(pipeline);
        }

        anyhow::bail!(
            "Pipeline '{}' not found. Looked for:\n  - {}\n  - {}",
            name,
            pipeline_path.display(),
            alt_path.display()
        );
    }

    let pipeline = Pipeline::from_file(&pipeline_path)?;
    pipeline.validate()?;
    Ok(pipeline)
}

// Fallback for atty if not available
mod atty {
    pub enum Stream {
        Stdin,
    }

    pub fn isnt(_stream: Stream) -> bool {
        // Simple heuristic: check if we're in a pipe
        // This is a simplified version - in production, use the atty crate
        true
    }
}
