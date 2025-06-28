use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn create_test_git_repo_with_config(commands: &[(&str, &str)]) -> PathBuf {
    let temp_dir = env::temp_dir().join(format!("iz-integration-test-{}", rand::random::<u32>()));
    fs::create_dir_all(&temp_dir).unwrap();
    let repo_path = &temp_dir;

    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Git init failed");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("Git config email failed");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("Git config name failed");
    let mut config_content = String::from("{\n  \"commands\": {\n");
    for (i, (key, value)) in commands.iter().enumerate() {
        config_content.push_str(&format!("    \"{key}\": \"{value}\""));
        if i < commands.len() - 1 {
            config_content.push(',');
        }
        config_content.push('\n');
    }
    config_content.push_str("  }\n}");

    fs::write(repo_path.join("izconfig.json"), config_content).unwrap();
    fs::write(repo_path.join("test.txt"), "Test content").unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Git add failed");

    Command::new("git")
        .args(["commit", "-m", "Test commit"])
        .current_dir(repo_path)
        .output()
        .expect("Git commit failed");

    temp_dir
}

fn get_iz_binary_path() -> String {
    use std::env;

    // First check if cargo test provides the binary path
    if let Ok(binary_path) = env::var("CARGO_BIN_EXE_iz") {
        if Path::new(&binary_path).exists() {
            return binary_path;
        }
    }

    let current_dir = env::current_dir().expect("Failed to get current directory");

    // Cross-platform executable name (adds .exe on Windows)
    let binary_name = format!("iz{}", std::env::consts::EXE_SUFFIX);

    let debug_path = current_dir.join("target/debug").join(&binary_name);
    if debug_path.exists() {
        return debug_path.to_string_lossy().to_string();
    }

    let release_path = current_dir.join("target/release").join(&binary_name);
    if release_path.exists() {
        return release_path.to_string_lossy().to_string();
    }

    panic!("iz CLI binary not found. Run 'cargo build' first.");
}

#[test]
fn test_iz_cli_basic_command() {
    let temp_repo = create_test_git_repo_with_config(&[
        ("hello", "echo 'Hello from test project!'"),
        ("pwd", "pwd"),
    ]);

    let iz_binary = get_iz_binary_path();

    let output = Command::new(&iz_binary)
        .args(["HEAD", "hello"])
        .current_dir(&temp_repo)
        .output()
        .expect("Failed to run iz CLI");

    assert!(
        output.status.success(),
        "iz CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from test project!"));
    assert!(stdout.contains("✅ Operation completed!"));
}

#[test]
fn test_iz_cli_with_parameters() {
    let temp_repo = create_test_git_repo_with_config(&[("greet", "echo 'Hello #{name}!'")]);

    let iz_binary = get_iz_binary_path();

    let output = Command::new(&iz_binary)
        .args(["HEAD", "greet", "--param", "name=Integration"])
        .current_dir(&temp_repo)
        .output()
        .expect("Failed to run iz CLI");

    assert!(
        output.status.success(),
        "iz CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello Integration!"));
}

#[test]
fn test_iz_cli_missing_config() {
    let temp_dir =
        env::temp_dir().join(format!("iz-test-missing-config-{}", rand::random::<u32>()));
    fs::create_dir_all(&temp_dir).unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .expect("Git init failed");

    let iz_binary = get_iz_binary_path();

    let output = Command::new(&iz_binary)
        .args(["HEAD", "test"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run iz CLI");

    assert!(!output.status.success(), "iz CLI should have failed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("izconfig.json not found"));
}

#[test]
fn test_iz_cli_missing_command() {
    let temp_repo = create_test_git_repo_with_config(&[("run", "echo 'run command'")]);

    let iz_binary = get_iz_binary_path();

    let output = Command::new(&iz_binary)
        .args(["HEAD", "nonexistent"])
        .current_dir(&temp_repo)
        .output()
        .expect("Failed to run iz CLI");

    assert!(!output.status.success(), "iz CLI should have failed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("nonexistent") && stderr.contains("not found"));
}

#[test]
fn test_iz_cli_help() {
    let iz_binary = get_iz_binary_path();

    let output = Command::new(&iz_binary)
        .args(["--help"])
        .output()
        .expect("Failed to run iz CLI help");

    assert!(output.status.success(), "iz CLI help failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CLI tool for testing Git commits in temporary directories"));
    assert!(stdout.contains("Usage:"));
}

#[test]
fn test_iz_cli_missing_parameter() {
    let temp_repo = create_test_git_repo_with_config(&[("greet", "echo 'Hello #{name}!'")]);

    let iz_binary = get_iz_binary_path();

    let output = Command::new(&iz_binary)
        .args(["HEAD", "greet"])
        .current_dir(&temp_repo)
        .output()
        .expect("Failed to run iz CLI");

    assert!(!output.status.success(), "iz CLI should have failed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Required parameter not found: name"));
}

#[test]
fn test_iz_cli_clean_force() {
    let temp_repo = create_test_git_repo_with_config(&[("test", "echo 'test'")]);
    let iz_binary = get_iz_binary_path();

    // Create fake temp directories
    let temp_base = temp_repo.join(".iztemp");
    fs::create_dir_all(&temp_base).unwrap();
    fs::create_dir_all(temp_base.join("iz-test1")).unwrap();
    fs::create_dir_all(temp_base.join("iz-test2")).unwrap();
    fs::create_dir_all(temp_base.join("other-folder")).unwrap();

    // Run cleanup with force
    let output = Command::new(&iz_binary)
        .args(["clean", "--force"])
        .current_dir(&temp_repo)
        .output()
        .expect("Failed to run iz clean");

    assert!(
        output.status.success(),
        "iz clean failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("🧹 Starting cleanup..."));
    assert!(stdout.contains("Found 2 temporary directories"));
    assert!(stdout.contains("Successfully cleaned 2 directories"));

    // Verify only iz- prefixed directories were cleaned
    assert!(!temp_base.join("iz-test1").exists());
    assert!(!temp_base.join("iz-test2").exists());
    assert!(temp_base.join("other-folder").exists());
}

#[test]
fn test_iz_cli_clean_no_directories() {
    let temp_repo = create_test_git_repo_with_config(&[("test", "echo 'test'")]);
    let iz_binary = get_iz_binary_path();

    // Create empty temp directory
    let temp_base = temp_repo.join(".iztemp");
    fs::create_dir_all(&temp_base).unwrap();

    let output = Command::new(&iz_binary)
        .args(["clean", "--force"])
        .current_dir(&temp_repo)
        .output()
        .expect("Failed to run iz clean");

    assert!(
        output.status.success(),
        "iz clean failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No temporary directories to clean"));
}

#[test]
fn test_iz_cli_clean_custom_temp_dir() {
    let temp_repo = create_test_git_repo_with_config(&[("test", "echo 'test'")]);
    let iz_binary = get_iz_binary_path();

    // Create custom temp directory with iz- directories
    let custom_temp = temp_repo.join("custom-temp");
    fs::create_dir_all(&custom_temp).unwrap();
    fs::create_dir_all(custom_temp.join("iz-custom1")).unwrap();
    fs::create_dir_all(custom_temp.join("iz-custom2")).unwrap();

    let output = Command::new(&iz_binary)
        .args(["clean", "--force", "--temp-dir", "custom-temp"])
        .current_dir(&temp_repo)
        .output()
        .expect("Failed to run iz clean");

    assert!(
        output.status.success(),
        "iz clean failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Successfully cleaned 2 directories"));

    // Verify directories were cleaned
    assert!(!custom_temp.join("iz-custom1").exists());
    assert!(!custom_temp.join("iz-custom2").exists());
}
