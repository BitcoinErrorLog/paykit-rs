//! Terminal UI utilities

use colored::Colorize;
use console::Term;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Print a success message
pub fn success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Print an error message
pub fn error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message);
}

/// Print an info message
pub fn info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

/// Print a warning message
pub fn warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message);
}

/// Print a section header
pub fn header(text: &str) {
    println!("\n{}", text.bold().underline());
}

/// Print a key-value pair
pub fn key_value(key: &str, value: &str) {
    println!("  {}: {}", key.cyan(), value);
}

/// Create a spinner progress indicator
pub fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Prompt for user confirmation
pub fn confirm(prompt: &str, default: bool) -> anyhow::Result<bool> {
    use dialoguer::Confirm;
    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()?)
}

/// Prompt for text input
pub fn input(prompt: &str) -> anyhow::Result<String> {
    use dialoguer::Input;
    Ok(Input::new().with_prompt(prompt).interact_text()?)
}

/// Prompt for text input with default
#[allow(dead_code)]
pub fn input_with_default(prompt: &str, default: &str) -> anyhow::Result<String> {
    use dialoguer::Input;
    Ok(Input::new()
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()?)
}

/// Display a QR code in the terminal
pub fn qr_code(data: &str) -> anyhow::Result<()> {
    use qrcode::QrCode;

    let code = QrCode::new(data)?;
    let string = code
        .render::<char>()
        .quiet_zone(false)
        .module_dimensions(2, 1)
        .build();

    println!("\n{}\n", string);
    Ok(())
}

/// Clear the terminal
#[allow(dead_code)]
pub fn clear() {
    let term = Term::stdout();
    let _ = term.clear_screen();
}

/// Print a separator line
pub fn separator() {
    println!("{}", "─".repeat(60).dimmed());
}

/// Print JSON prettily
pub fn json(value: &serde_json::Value) {
    if let Ok(pretty) = serde_json::to_string_pretty(value) {
        println!("{}", pretty);
    }
}
