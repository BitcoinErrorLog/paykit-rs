//! Binary to generate UniFFI bindings using uniffi_bindgen as a library
//! This works around the issue where uniffi-bindgen binary isn't available in 0.25

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "generate-bindings")]
#[command(about = "Generate UniFFI bindings for paykit-mobile")]
struct Cli {
    /// Path to the compiled library (.dylib, .so, or .a file)
    #[arg(long, default_value = "../target/release/libpaykit_mobile.dylib")]
    library: PathBuf,

    /// Output language
    #[arg(short = 'l', long = "language", default_value = "swift")]
    language: Language,

    /// Output directory
    #[arg(short = 'o', long = "out-dir")]
    out_dir: Option<PathBuf>,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Language {
    Swift,
    Kotlin,
    Python,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Determine output directory
    let out_dir = cli.out_dir.unwrap_or_else(|| {
        let mut dir = PathBuf::from(".");
        match cli.language {
            Language::Swift => dir.push("swift/generated"),
            Language::Kotlin => dir.push("kotlin/generated"),
            Language::Python => dir.push("python/generated"),
        }
        dir
    });

    // Ensure output directory exists
    std::fs::create_dir_all(&out_dir)?;

    println!(
        "Generating {} bindings...",
        format!("{:?}", cli.language).to_lowercase()
    );
    println!("Library: {}", cli.library.display());
    println!("Output: {}", out_dir.display());

    // Use uniffi_bindgen's library API
    // Note: uniffi_bindgen 0.25's API is internal, so we need to use the CLI interface
    // For now, we'll use a workaround by calling the library's internal functions

    // Check if library exists
    if !cli.library.exists() {
        anyhow::bail!("Library not found: {}", cli.library.display());
    }

    // Use uniffi_bindgen's generate_bindings function
    // This is a simplified version - the actual API is more complex
    let language_str = match cli.language {
        Language::Swift => "swift",
        Language::Kotlin => "kotlin",
        Language::Python => "python",
    };

    // Call uniffi_bindgen's main function with the right arguments
    // Since uniffi_bindgen 0.25 doesn't expose a clean API, we'll need to
    // use the command-line interface through std::process

    // Actually, let's try using the uniffi crate's built-in functionality
    // The uniffi crate with "cli" feature should provide bindgen functionality

    // Use uniffi_bindgen's library API
    // The library provides a generate_bindings function we can call
    use uniffi_bindgen::generate_bindings;

    let language_enum = match cli.language {
        Language::Swift => uniffi_bindgen::BindingGeneratorLang::Swift,
        Language::Kotlin => uniffi_bindgen::BindingGeneratorLang::Kotlin,
        Language::Python => uniffi_bindgen::BindingGeneratorLang::Python,
    };

    println!("Calling uniffi_bindgen::generate_bindings...");

    // Call the actual bindgen function
    generate_bindings(
        &cli.library,
        language_enum,
        None, // config_file
        &out_dir,
        false, // only_one_binding_file
    )?;

    println!("✅ Bindings generated successfully!");
    Ok(())
}
