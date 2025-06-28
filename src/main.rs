use anyhow::{Context, Result};
use clap::Parser;
use git2::Repository;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
#[cfg(unix)]
use tokio::signal;

use iz::{parse_key_val, read_config, substitute_variables};

static CLEANUP_STATE: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

#[derive(Parser)]
#[command(
    name = "iz",
    about = "CLI tool for testing Git commits in temporary directories",
    version = "0.1.0"
)]
struct Cli {
    /// Git commit ID or 'clean' command
    commit_id_or_command: String,

    /// Command to execute (not needed for 'clean')
    command: Option<String>,

    /// Keep temporary directory after execution
    #[arg(long)]
    keep: bool,

    /// Temporary directory path (default: .iztemp)
    #[arg(long)]
    temp_dir: Option<String>,

    /// Additional parameters (--key=value format)
    #[arg(long, value_parser = parse_key_val)]
    param: Vec<(String, String)>,

    /// Force operation without confirmation (for clean command)
    #[arg(long)]
    force: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check if first argument is "clean" command
    if cli.commit_id_or_command == "clean" {
        return clean_command(cli.temp_dir, cli.force).await;
    }

    // Original behavior for commit ID + command
    let commit_id = cli.commit_id_or_command;
    let command = cli.command.ok_or_else(|| {
        anyhow::anyhow!("Command is required. Usage: iz <commit-id> <command> or iz clean")
    })?;

    run_command(commit_id, command, cli.keep, cli.temp_dir, cli.param).await
}

async fn run_command(
    commit_id: String,
    command: String,
    keep: bool,
    temp_dir: Option<String>,
    param: Vec<(String, String)>,
) -> Result<()> {
    println!("🔄 Starting iz CLI...");

    let config = read_config().context("Failed to read izconfig.json")?;

    let command_template = config
        .commands
        .get(&command)
        .ok_or_else(|| anyhow::anyhow!("Command '{}' not found in izconfig.json", command))?;

    let params: HashMap<String, String> = param.into_iter().collect();
    let final_command = substitute_variables(command_template, &params)?;

    println!("🎯 Commit: {}", commit_id);
    println!("📝 Command: {final_command}");

    let should_keep = keep || config.keep.unwrap_or(false);
    let base_temp_dir = determine_temp_dir(&temp_dir, &config)?;
    let temp_path = create_unique_temp_dir(&base_temp_dir)?;

    if !should_keep {
        let mut cleanup_state = CLEANUP_STATE.lock().unwrap();
        *cleanup_state = Some(temp_path.clone());
    }

    println!("📁 Temporary directory: {}", temp_path.display());

    let signal_handle = if !should_keep {
        Some(tokio::spawn(async {
            let _ = setup_signal_handler().await;
        }))
    } else {
        None
    };

    checkout_commit_to_temp(&commit_id, &temp_path).context("Failed to checkout commit")?;

    println!("🚀 Executing command...");
    execute_command(&final_command, &temp_path).context("Failed to execute command")?;

    cleanup_temp_directory(&temp_path, should_keep);

    if let Some(handle) = signal_handle {
        handle.abort();
    }

    println!("✅ Operation completed!");
    Ok(())
}

async fn clean_command(temp_dir: Option<String>, force: bool) -> Result<()> {
    println!("🧹 Starting cleanup...");

    let config = read_config().context("Failed to read izconfig.json")?;
    let base_temp_dir = determine_temp_dir(&temp_dir, &config)?;

    if !base_temp_dir.exists() {
        println!(
            "📁 Temporary directory does not exist: {}",
            base_temp_dir.display()
        );
        return Ok(());
    }

    // List contents of temp directory
    let entries = fs::read_dir(&base_temp_dir)
        .with_context(|| format!("Failed to read temp directory: {}", base_temp_dir.display()))?;

    let mut items_to_clean = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir()
            && path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .starts_with("iz-")
        {
            items_to_clean.push(path);
        }
    }

    if items_to_clean.is_empty() {
        println!(
            "✨ No temporary directories to clean in: {}",
            base_temp_dir.display()
        );
        return Ok(());
    }

    println!("📋 Found {} temporary directories:", items_to_clean.len());
    for item in &items_to_clean {
        println!("  • {}", item.display());
    }

    if !force {
        print!("❓ Do you want to clean these directories? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("🚫 Cleanup cancelled");
            return Ok(());
        }
    }

    let mut cleaned_count = 0;
    let mut failed_count = 0;

    for item in items_to_clean {
        match fs::remove_dir_all(&item) {
            Ok(()) => {
                cleaned_count += 1;
                println!("✅ Cleaned: {}", item.display());
            }
            Err(e) => {
                failed_count += 1;
                eprintln!("❌ Failed to clean {}: {}", item.display(), e);
            }
        }
    }

    if failed_count == 0 {
        println!("🎉 Successfully cleaned {} directories!", cleaned_count);
    } else {
        println!(
            "⚠️  Cleaned {} directories, {} failed",
            cleaned_count, failed_count
        );
    }

    Ok(())
}

fn checkout_commit_to_temp(commit_id: &str, temp_path: &std::path::Path) -> Result<()> {
    let repo = Repository::open(std::env::current_dir()?)
        .context("Git repository not found - this directory is not a git repository")?;

    let object = repo
        .revparse_single(commit_id)
        .context("Commit not found - invalid commit ID")?;

    let commit = object
        .peel_to_commit()
        .context("Given reference does not point to a commit")?;

    let tree = commit.tree().context("Failed to get commit tree")?;

    // Pre-create directory structure to avoid git2 checkout issues
    create_directory_structure(&tree, temp_path).context("Failed to create directory structure")?;

    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.target_dir(temp_path);
    checkout_builder.force();
    checkout_builder.recreate_missing(true);

    repo.checkout_tree(tree.as_object(), Some(&mut checkout_builder))
        .context("Failed to extract files")?;

    Ok(())
}

fn create_directory_structure(tree: &git2::Tree, base_path: &std::path::Path) -> Result<()> {
    tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if let Some(git2::ObjectType::Tree) = entry.kind() {
            let dir_path = base_path.join(root).join(entry.name().unwrap_or(""));
            if let Err(e) = fs::create_dir_all(&dir_path) {
                eprintln!(
                    "Warning: Failed to create directory {}: {}",
                    dir_path.display(),
                    e
                );
            }
        }
        git2::TreeWalkResult::Ok
    })?;

    Ok(())
}

fn execute_command(command: &str, working_dir: &std::path::Path) -> Result<()> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty command"));
    }

    let mut cmd = Command::new(parts[0]);
    if parts.len() > 1 {
        cmd.args(&parts[1..]);
    }

    cmd.current_dir(working_dir);

    let output = cmd.output().context("Failed to execute command")?;

    if !output.stdout.is_empty() {
        println!("📄 Output:");
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }

    if !output.stderr.is_empty() {
        eprintln!("⚠️  Error output:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Command failed with status: {}",
            output.status
        ));
    }

    Ok(())
}

#[cfg(unix)]
async fn setup_signal_handler() -> Result<()> {
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;

    tokio::select! {
        _ = sigint.recv() => {
            println!("\n🛑 Received SIGINT (Ctrl+C)");
            perform_cleanup();
            std::process::exit(130);
        }
        _ = sigterm.recv() => {
            println!("\n🛑 Received SIGTERM");
            perform_cleanup();
            std::process::exit(143);
        }
    }
}

#[cfg(windows)]
async fn setup_signal_handler() -> Result<()> {
    tokio::signal::ctrl_c().await?;
    println!("\n🛑 Received Ctrl+C");
    perform_cleanup();
    std::process::exit(130);
}

fn perform_cleanup() {
    if let Ok(mut cleanup_state) = CLEANUP_STATE.lock() {
        if let Some(temp_path) = cleanup_state.take() {
            if let Err(e) = fs::remove_dir_all(&temp_path) {
                eprintln!("⚠️  Error during signal cleanup: {e}");
            } else {
                println!("🧹 Temporary directory cleaned up: {}", temp_path.display());
            }
        }
    }
}

fn cleanup_temp_directory(temp_path: &PathBuf, should_keep: bool) {
    if let Ok(mut cleanup_state) = CLEANUP_STATE.lock() {
        *cleanup_state = None;
    }

    if should_keep {
        println!("💾 Temporary directory preserved: {}", temp_path.display());
    } else if let Err(e) = fs::remove_dir_all(temp_path) {
        eprintln!("⚠️  Error cleaning temporary directory: {e}");
    } else {
        println!("🧹 Temporary directory cleaned");
    }
}

fn determine_temp_dir(cli_temp_dir: &Option<String>, config: &iz::IzConfig) -> Result<PathBuf> {
    if let Some(temp_dir) = cli_temp_dir {
        return Ok(PathBuf::from(temp_dir));
    }

    if let Ok(env_temp_dir) = std::env::var("IZTEMP") {
        return Ok(PathBuf::from(env_temp_dir));
    }

    if let Some(config_temp_dir) = &config.temp_dir {
        return Ok(PathBuf::from(config_temp_dir));
    }

    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    Ok(current_dir.join(".iztemp"))
}

fn create_unique_temp_dir(base_temp_dir: &PathBuf) -> Result<PathBuf> {
    if !base_temp_dir.exists() {
        fs::create_dir_all(base_temp_dir).with_context(|| {
            format!(
                "Failed to create temp directory: {}",
                base_temp_dir.display()
            )
        })?;
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let random_id: u32 = rand::random();
    let unique_name = format!("iz-{timestamp}-{random_id:x}");

    let temp_path = base_temp_dir.join(unique_name);

    fs::create_dir_all(&temp_path).with_context(|| {
        format!(
            "Failed to create temporary directory: {}",
            temp_path.display()
        )
    })?;

    Ok(temp_path)
}
