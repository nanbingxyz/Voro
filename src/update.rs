use anyhow::{bail, Context, Result};
use crossterm::{
    cursor::MoveToColumn,
    execute,
    style::Print,
    terminal::{Clear, ClearType},
};
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tar::Archive;
use zip::ZipArchive;

const GITHUB_API_URL: &str = "https://api.github.com/repos";
const GITHUB_REPO: &str = "nanbingxyz/voro";

/// GitHub release response
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[allow(dead_code)]
    assets: Vec<Asset>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Current version from Cargo.toml
pub fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get platform identifier for downloading
fn get_platform() -> Result<&'static str> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok("darwin-arm64");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Ok("darwin-x64");

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok("linux-x64");

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Ok("windows-x64");

    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    bail!("Unsupported platform. Supported: macOS (arm64/x64), Linux x64, Windows x64")
}

/// Spinner animation frames
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Loading animation runner
struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    fn new(message: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let message = message.to_string();

        let handle = thread::spawn(move || {
            let mut frame = 0;
            while running_clone.load(Ordering::Relaxed) {
                let spinner = SPINNER_FRAMES[frame % SPINNER_FRAMES.len()];
                let _ = execute!(
                    io::stdout(),
                    Clear(ClearType::CurrentLine),
                    MoveToColumn(0),
                    Print(format!("\x1b[36m{}\x1b[0m {}", spinner, message))
                );
                let _ = io::stdout().flush();
                frame += 1;
                thread::sleep(Duration::from_millis(80));
            }
        });

        Spinner {
            running,
            handle: Some(handle),
        }
    }

    fn finish(self, message: &str) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle {
            handle.join().ok();
        }
        let _ = execute!(
            io::stdout(),
            Clear(ClearType::CurrentLine),
            MoveToColumn(0),
            Print(format!("\x1b[32m✓\x1b[0m {}", message))
        );
        println!();
    }

    fn fail(self, message: &str) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle {
            handle.join().ok();
        }
        let _ = execute!(
            io::stdout(),
            Clear(ClearType::CurrentLine),
            MoveToColumn(0),
            Print(format!("\x1b[31m✗\x1b[0m {}", message))
        );
        println!();
    }
}

/// Check for updates and return the latest version if available
pub fn check_for_update() -> Result<Option<String>> {
    let spinner = Spinner::new("Checking for updates...");

    let url = format!("{}/{}/releases/latest", GITHUB_API_URL, GITHUB_REPO);

    let response = ureq::get(&url)
        .set("Accept", "application/vnd.github.v3+json")
        .set("User-Agent", "voro-update-checker")
        .call();

    match response {
        Ok(response) => {
            let release: Result<Release, _> = response.into_json();
            match release {
                Ok(release) => {
                    let current = current_version();
                    let latest = release.tag_name.trim_start_matches('v').to_string();

                    spinner.finish(&format!("Current: v{}, Latest: v{}", current, latest));

                    if latest != current {
                        Ok(Some(release.tag_name))
                    } else {
                        println!("Already up to date!");
                        Ok(None)
                    }
                }
                Err(e) => {
                    spinner.fail(&format!("Failed to parse release info: {}", e));
                    bail!("Failed to parse release info: {}", e)
                }
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("404") {
                spinner.fail("No releases found. Please create a release first.");
                bail!("No releases found at {}", GITHUB_REPO);
            }
            spinner.fail(&format!("Failed to check for updates: {}", e));
            bail!("Failed to check for updates: {}", e)
        }
    }
}

/// Download and install the update
pub fn perform_update(version: &str) -> Result<()> {
    let platform = get_platform()?;
    let ext = if cfg!(windows) { "zip" } else { "tar.gz" };
    let asset_name = format!("voro-{}-{}.{}", version, platform, ext);

    let download_url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        GITHUB_REPO, version, asset_name
    );

    // Download
    let spinner = Spinner::new(&format!("Downloading {}...", asset_name));

    let response = ureq::get(&download_url)
        .set("User-Agent", "voro-updater")
        .call()
        .context("Failed to download update")?;

    let mut reader = response.into_reader();
    let temp_dir = env::temp_dir();
    let archive_path = temp_dir.join(&asset_name);
    let mut archive_file = File::create(&archive_path)?;
    io::copy(&mut reader, &mut archive_file)?;

    spinner.finish(&format!("Downloaded {}", asset_name));

    // Extract
    let spinner = Spinner::new("Extracting binary...");

    let binary_name = if cfg!(windows) { "vo.exe" } else { "vo" };
    let extracted_binary = extract_binary(&archive_path, binary_name)?;

    spinner.finish("Extraction complete");

    // Install
    let spinner = Spinner::new("Installing update...");

    let current_exe = env::current_exe()?;
    install_binary(&extracted_binary, &current_exe)?;

    // Cleanup
    fs::remove_file(&archive_path).ok();
    fs::remove_file(&extracted_binary).ok();

    spinner.finish(&format!("Successfully updated to {}!", version));

    Ok(())
}

/// Extract binary from archive
fn extract_binary(archive_path: &Path, binary_name: &str) -> Result<PathBuf> {
    let temp_dir = env::temp_dir();
    let output_path = temp_dir.join(binary_name);

    if archive_path.extension().map_or(false, |e| e == "zip") {
        // Extract from zip (Windows)
        let file = File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut binary = archive.by_name(binary_name)?;
        let mut output = File::create(&output_path)?;
        io::copy(&mut binary, &mut output)?;
    } else {
        // Extract from tar.gz (Unix)
        let file = File::open(archive_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            if path.file_name().map_or(false, |f| f == binary_name) {
                let mut output = File::create(&output_path)?;
                io::copy(&mut entry, &mut output)?;
                break;
            }
        }
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&output_path, fs::Permissions::from_mode(0o755))?;
    }

    Ok(output_path)
}

/// Install binary by replacing current executable
fn install_binary(new_binary: &Path, current_exe: &Path) -> Result<()> {
    // On Windows, we can't replace a running executable directly
    // We rename the old one and then move the new one
    #[cfg(windows)]
    {
        let old_path = current_exe.with_extension("exe.old");
        fs::rename(current_exe, &old_path)?;
        fs::rename(new_binary, current_exe)?;
        fs::remove_file(&old_path).ok(); // Clean up on next run if this fails
    }

    #[cfg(unix)]
    {
        fs::rename(new_binary, current_exe)?;
    }

    Ok(())
}

/// Run the update command
pub fn run_update() -> Result<()> {
    println!("\x1b[1;36mvoro update\x1b[0m\n");

    // Check for update
    let Some(version) = check_for_update()? else {
        return Ok(());
    };

    println!("\nNew version {} available!", version);
    print!("Update now? (y/n): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
        perform_update(&version)?;
    } else {
        println!("Update cancelled.");
    }

    Ok(())
}
