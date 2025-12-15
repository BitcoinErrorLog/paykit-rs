//! Binary to generate UniFFI bindings using uniffi_bindgen as a library
//! Updated for uniffi 0.29.x API

use camino::Utf8PathBuf;
use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(name = "generate-bindings")]
#[command(about = "Generate UniFFI bindings for paykit-mobile")]
struct Cli {
    /// Path to the compiled library (.dylib, .so, or .a file)
    #[arg(long, default_value = "../target/release/libpaykit_mobile.dylib")]
    library: Utf8PathBuf,

    /// Output language
    #[arg(short = 'l', long = "language", default_value = "swift")]
    language: Language,

    /// Output directory
    #[arg(short = 'o', long = "out-dir")]
    out_dir: Option<Utf8PathBuf>,
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
        let mut dir = Utf8PathBuf::from(".");
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
    println!("Library: {}", cli.library);
    println!("Output: {}", out_dir);

    // Check if library exists
    if !cli.library.exists() {
        anyhow::bail!("Library not found: {}", cli.library);
    }

    // Use uniffi_bindgen's library_mode API for 0.29.x
    use uniffi_bindgen::bindings::{KotlinBindingGenerator, PythonBindingGenerator, SwiftBindingGenerator};
    use uniffi_bindgen::library_mode::generate_bindings;

    println!("Calling uniffi_bindgen::library_mode::generate_bindings...");

    match cli.language {
        Language::Swift => {
            generate_bindings(
                &cli.library,
                None,
                &SwiftBindingGenerator,
                &uniffi_bindgen::EmptyCrateConfigSupplier,
                None,
                &out_dir,
                false,
            )?;
        }
        Language::Kotlin => {
            generate_bindings(
                &cli.library,
                None,
                &KotlinBindingGenerator,
                &uniffi_bindgen::EmptyCrateConfigSupplier,
                None,
                &out_dir,
                false,
            )?;
        }
        Language::Python => {
            generate_bindings(
                &cli.library,
                None,
                &PythonBindingGenerator,
                &uniffi_bindgen::EmptyCrateConfigSupplier,
                None,
                &out_dir,
                false,
            )?;
        }
    }

    println!("✅ Bindings generated successfully!");
    Ok(())
}
