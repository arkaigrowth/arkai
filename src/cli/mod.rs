//! Command-line interface for arkai.
//!
//! Provides commands for running pipelines, checking status,
//! listing runs, resuming failed runs, and managing the content library.

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use uuid::Uuid;

use crate::adapters::{Adapter, FabricAdapter, ACTION_WEB, ACTION_YOUTUBE};
use crate::core::{Orchestrator, Pipeline};
use crate::library::{Catalog, CatalogItem, ContentType, LibraryContent};

pub mod evidence;

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

    /// Ingest content from a URL (YouTube or web)
    Ingest {
        /// URL to ingest
        url: String,

        /// Content type (auto-detected if not specified)
        #[arg(short, long, value_enum)]
        content_type: Option<IngestType>,

        /// Tags to apply (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Custom title (extracted from content if not specified)
        #[arg(long)]
        title: Option<String>,
    },

    /// List items in the library
    Library {
        /// Filter by content type
        #[arg(short, long, value_enum)]
        content_type: Option<IngestType>,

        /// Maximum number of items to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show resolved configuration (debug)
    Config,

    /// Search the library
    Search {
        /// Search query
        query: String,
    },

    /// Show details of a library item
    Show {
        /// Content ID
        content_id: String,

        /// Show full artifact content
        #[arg(short, long)]
        full: bool,
    },

    /// Reprocess a library item
    Reprocess {
        /// Content ID to reprocess
        content_id: String,
    },

    /// Run a Fabric pattern directly
    Pattern {
        /// Pattern name (e.g., "astro", "extract_wisdom", "summarize")
        pattern_name: String,

        /// Input file (reads from stdin if not provided)
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Save output to library with this title
        #[arg(short, long)]
        save: Option<String>,

        /// Tags to apply when saving (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// Manage evidence and provenance
    Evidence {
        #[command(subcommand)]
        command: evidence::EvidenceCommands,
    },
}

/// Content type for CLI (maps to ContentType)
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum IngestType {
    /// YouTube video
    Youtube,

    /// Web page/article
    Web,
}

impl From<IngestType> for ContentType {
    fn from(t: IngestType) -> Self {
        match t {
            IngestType::Youtube => ContentType::YouTube,
            IngestType::Web => ContentType::Web,
        }
    }
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
            Commands::Ingest {
                url,
                content_type,
                tags,
                title,
            } => {
                ingest_content(&url, content_type, tags, title).await
            }
            Commands::Config => {
                show_config().await
            }
            Commands::Library { content_type, limit } => {
                list_library(content_type, limit).await
            }
            Commands::Search { query } => {
                search_library(&query).await
            }
            Commands::Show { content_id, full } => {
                show_content(&content_id, full).await
            }
            Commands::Reprocess { content_id } => {
                reprocess_content(&content_id).await
            }
            Commands::Pattern {
                pattern_name,
                input,
                save,
                tags,
            } => {
                run_pattern(&pattern_name, input, save, tags).await
            }
            Commands::Evidence { command } => {
                execute_evidence(command).await
            }
        }
    }
}

/// Execute evidence subcommands
async fn execute_evidence(command: evidence::EvidenceCommands) -> Result<()> {
    match command {
        evidence::EvidenceCommands::Show { evidence_id } => {
            evidence::execute_show(&evidence_id).await
        }
        evidence::EvidenceCommands::Open { evidence_id } => {
            evidence::execute_open(&evidence_id).await
        }
        evidence::EvidenceCommands::Validate { content_id } => {
            evidence::execute_validate(&content_id).await
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

/// Detect content type from URL
fn detect_content_type(url: &str) -> ContentType {
    let url_lower = url.to_lowercase();
    if url_lower.contains("youtube.com") || url_lower.contains("youtu.be") {
        ContentType::YouTube
    } else {
        ContentType::Web
    }
}

/// Get YouTube video title using yt-dlp
/// Returns None if not a YouTube URL or if yt-dlp fails
fn get_youtube_title(url: &str) -> Option<String> {
    let url_lower = url.to_lowercase();
    if !url_lower.contains("youtube.com") && !url_lower.contains("youtu.be") {
        return None;
    }

    let output = Command::new("yt-dlp")
        .args(["--get-title", "--no-warnings", url])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    }
}

/// Extract title from content (first non-empty line or fallback to URL)
fn extract_title(content: &str, url: &str) -> String {
    // First try to get YouTube title via yt-dlp (fast metadata lookup)
    if let Some(title) = get_youtube_title(url) {
        return title;
    }

    // Try to extract title from first line (often contains title in transcripts)
    content
        .lines()
        .find(|line| !line.trim().is_empty() && line.len() < 200)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            // Fallback: extract from URL
            url.split('/')
                .last()
                .unwrap_or("Untitled")
                .split('?')
                .next()
                .unwrap_or("Untitled")
                .to_string()
        })
}

/// Create a dynamic ingestion pipeline
fn create_ingest_pipeline(content_type: ContentType) -> Pipeline {
    use crate::core::pipeline::{AdapterType, InputSource, PipelineInputMarker, RetryPolicy, Step};
    use crate::core::safety::SafetyLimits;

    let (name, fetch_action) = match content_type {
        ContentType::YouTube => ("youtube-wisdom", ACTION_YOUTUBE),
        ContentType::Web => ("web-wisdom", ACTION_WEB),
        ContentType::Other => ("content-wisdom", ACTION_WEB),
    };

    Pipeline {
        name: name.to_string(),
        description: format!("{} content ingestion pipeline", name),
        safety_limits: SafetyLimits {
            step_timeout_seconds: 120, // 2 minutes for fetching
            ..Default::default()
        },
        steps: vec![
            Step {
                name: "fetch".to_string(),
                adapter: AdapterType::Fabric,
                action: fetch_action.to_string(),
                input_from: InputSource::PipelineInput(PipelineInputMarker::PipelineInput),
                retry_policy: RetryPolicy::default(),
                timeout_seconds: Some(120),
            },
            Step {
                name: "wisdom".to_string(),
                adapter: AdapterType::Fabric,
                action: "extract_wisdom".to_string(),
                input_from: InputSource::PreviousStep {
                    previous_step: "fetch".to_string(),
                },
                retry_policy: RetryPolicy::default(),
                timeout_seconds: Some(180),
            },
            Step {
                name: "summary".to_string(),
                adapter: AdapterType::Fabric,
                action: "summarize".to_string(),
                input_from: InputSource::PreviousStep {
                    previous_step: "wisdom".to_string(),
                },
                retry_policy: RetryPolicy::default(),
                timeout_seconds: Some(120),
            },
        ],
    }
}

/// Ingest content from a URL
async fn ingest_content(
    url: &str,
    content_type: Option<IngestType>,
    tags: Option<String>,
    title: Option<String>,
) -> Result<()> {
    // Detect or use specified content type
    let ct = content_type
        .map(ContentType::from)
        .unwrap_or_else(|| detect_content_type(url));

    eprintln!("📥 Ingesting {} content from: {}", ct, url);

    // Create dynamic pipeline for ingestion
    let pipeline = create_ingest_pipeline(ct);

    // Run the pipeline with URL as input
    let orchestrator = Orchestrator::new();
    let run = orchestrator.run_pipeline(&pipeline, url.to_string()).await?;

    match &run.state {
        crate::domain::RunState::Completed => {
            // Get the fetch output for title extraction if not provided
            let final_title = title.unwrap_or_else(|| {
                run.artifacts
                    .get("fetch")
                    .map(|a| extract_title(&a.content, url))
                    .unwrap_or_else(|| extract_title("", url))
            });

            // Create library content
            let content = LibraryContent::new(url, &final_title, ct);

            // Copy artifacts from run to library
            let artifacts = content.copy_from_run(run.id).await?;
            content.save_metadata().await?;

            // Update catalog
            let mut catalog = Catalog::load().await?;
            let mut item = CatalogItem::new(url, &final_title, ct)
                .with_run_id(run.id.to_string());

            // Add tags
            if let Some(tags_str) = tags {
                let tag_list: Vec<String> = tags_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                item = item.with_tags(tag_list);
            }

            // Add artifacts
            for artifact in &artifacts {
                item = item.with_artifact(artifact.clone());
            }

            catalog.add(item);
            catalog.save().await?;

            eprintln!("\n✅ Content ingested successfully!");
            eprintln!("   ID: {}", content.id);
            eprintln!("   Title: {}", final_title);
            eprintln!("   Artifacts: {:?}", artifacts);
            eprintln!("   Run: {}", run.id);

            // Print the wisdom output
            if let Some(wisdom) = run.artifacts.get("wisdom") {
                println!("\n{}", wisdom.content);
            }
        }
        crate::domain::RunState::Failed { error } => {
            eprintln!("\n❌ Ingestion failed: {}", error);
            eprintln!("   Run: {}", run.id);
            std::process::exit(1);
        }
        _ => {
            eprintln!("\n⚠️ Ingestion ended in unexpected state: {:?}", run.state);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// List items in the library
async fn list_library(content_type: Option<IngestType>, limit: usize) -> Result<()> {
    let catalog = Catalog::load().await?;

    if catalog.is_empty() {
        println!("Library is empty. Use 'arkai ingest <url>' to add content.");
        return Ok(());
    }

    let items: Vec<&CatalogItem> = if let Some(ct) = content_type {
        catalog.filter_by_type(ct.into())
    } else {
        catalog.list(Some(limit))
    };

    println!("{:<18} {:<10} {:<50}", "ID", "TYPE", "TITLE");
    println!("{}", "-".repeat(80));

    for item in items.iter().take(limit) {
        let title_truncated = if item.title.len() > 47 {
            format!("{}...", &item.title[..47])
        } else {
            item.title.clone()
        };
        println!(
            "{:<18} {:<10} {:<50}",
            item.id.as_str(),
            item.content_type.to_string(),
            title_truncated
        );
    }

    println!("\nTotal: {} items", catalog.len());

    Ok(())
}

/// Search the library
async fn search_library(query: &str) -> Result<()> {
    let catalog = Catalog::load().await?;

    let results = catalog.search(query);

    if results.is_empty() {
        println!("No results found for: {}", query);
        return Ok(());
    }

    println!("Found {} result(s) for \"{}\":\n", results.len(), query);
    println!("{:<18} {:<10} {:<50}", "ID", "TYPE", "TITLE");
    println!("{}", "-".repeat(80));

    for item in &results {
        let title_truncated = if item.title.len() > 47 {
            format!("{}...", &item.title[..47])
        } else {
            item.title.clone()
        };
        println!(
            "{:<18} {:<10} {:<50}",
            item.id.as_str(),
            item.content_type.to_string(),
            title_truncated
        );
    }

    Ok(())
}

/// Show details of a library item
async fn show_content(content_id: &str, full: bool) -> Result<()> {
    let catalog = Catalog::load().await?;

    // Find the item by ID prefix match
    let item = catalog
        .items
        .iter()
        .find(|i| i.id.as_str().starts_with(content_id))
        .ok_or_else(|| anyhow::anyhow!("Content not found: {}", content_id))?;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("  ID: {}", item.id);
    println!("  Title: {}", item.title);
    println!("  URL: {}", item.url);
    println!("  Type: {}", item.content_type);
    println!("  Processed: {}", item.processed_at);
    if !item.tags.is_empty() {
        println!("  Tags: {}", item.tags.join(", "));
    }
    println!("  Artifacts: {:?}", item.artifacts);
    if let Some(run_id) = &item.run_id {
        println!("  Run ID: {}", run_id);
    }
    println!("╚══════════════════════════════════════════════════════════════╝");

    if full {
        // Load and display artifacts
        let content = LibraryContent::load_metadata(&item.id).await?;

        for artifact_name in &item.artifacts {
            if let Some(artifact_content) = content.load_artifact(artifact_name).await? {
                println!("\n═══ {} ═══\n", artifact_name.to_uppercase());
                println!("{}", artifact_content);
            }
        }
    } else {
        println!("\nUse --full to show artifact contents");
    }

    Ok(())
}

/// Reprocess a library item
async fn reprocess_content(content_id: &str) -> Result<()> {
    let catalog = Catalog::load().await?;

    // Find the item by ID prefix match
    let item = catalog
        .items
        .iter()
        .find(|i| i.id.as_str().starts_with(content_id))
        .ok_or_else(|| anyhow::anyhow!("Content not found: {}", content_id))?;

    eprintln!("🔄 Reprocessing: {}", item.title);
    eprintln!("   URL: {}", item.url);

    // Re-ingest with the same URL
    ingest_content(&item.url, None, None, Some(item.title.clone())).await
}

/// Show the resolved configuration (for debugging)
async fn show_config() -> Result<()> {
    use crate::config;
    use crate::library::ContentType;

    let cfg = config::config()?;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("  ArkAI Configuration");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Config file: {}", cfg.config_file.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "(none - using defaults)".to_string()));
    println!();
    println!("Paths:");
    println!("  Home (engine state): {}", cfg.home.display());
    println!("  Library (content):   {}", cfg.library.display());
    println!("  Runs:                {}", cfg.home.join("runs").display());
    println!("  Catalog:             {}", cfg.home.join("catalog.json").display());
    println!();
    println!("Content type directories:");
    println!("  YouTube:  {}", config::content_type_dir(ContentType::YouTube)?.display());
    println!("  Web:      {}", config::content_type_dir(ContentType::Web)?.display());
    println!("  Other:    {}", config::content_type_dir(ContentType::Other)?.display());
    println!();
    println!("Content type mappings:");
    if cfg.content_types.is_empty() {
        println!("  (using defaults)");
    } else {
        for (k, v) in &cfg.content_types {
            println!("  {}: {}", k, v);
        }
    }
    println!();
    println!("Safety limits:");
    println!("  Max steps:      {}", cfg.safety.max_steps);
    println!("  Timeout:        {}s", cfg.safety.timeout_seconds);
    println!("  Max input size: {} bytes", cfg.safety.max_input_size_bytes);

    Ok(())
}

/// Run a Fabric pattern directly
async fn run_pattern(
    pattern_name: &str,
    input_file: Option<PathBuf>,
    save_title: Option<String>,
    tags: Option<String>,
) -> Result<()> {
    use std::time::Duration;

    // Get input from file or stdin
    let input = if let Some(path) = input_file {
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read input file: {}", path.display()))?
    } else {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        buffer
    };

    if input.trim().is_empty() {
        anyhow::bail!("No input provided. Use --input <file> or pipe to stdin");
    }

    eprintln!("🔮 Running pattern: {}", pattern_name);

    // Execute the pattern via Fabric adapter
    let adapter = FabricAdapter::new();
    let timeout = Duration::from_secs(300); // 5 minutes for patterns

    let output = adapter
        .execute(pattern_name, &input, timeout)
        .await
        .with_context(|| format!("Failed to run pattern '{}'", pattern_name))?;

    // Print the output
    println!("{}", output.content);

    // Optionally save to library
    if let Some(title) = save_title {
        eprintln!("\n📚 Saving to library...");

        // Create a unique ID for the pattern output
        let content_id = format!(
            "pattern-{}-{}",
            pattern_name,
            &uuid::Uuid::new_v4().to_string()[..8]
        );

        // Update catalog
        let mut catalog = Catalog::load().await?;
        let mut item = CatalogItem::new(
            &format!("pattern://{}", pattern_name),
            &title,
            ContentType::Other,
        );

        // Add tags if provided
        if let Some(tags_str) = tags {
            let tag_list: Vec<String> = tags_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            item = item.with_tags(tag_list);
        }

        // Add the pattern name as a tag too
        item.tags.push(format!("pattern:{}", pattern_name));

        catalog.add(item);
        catalog.save().await?;

        eprintln!("   ID: {}", content_id);
        eprintln!("   Title: {}", title);
    }

    Ok(())
}
