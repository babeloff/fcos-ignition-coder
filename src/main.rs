use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ignition_config::Config;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "fcos-ignition-coder")]
#[command(about = "Decode and encode Fedora CoreOS Ignition configuration files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Decode an Ignition file, extracting embedded files
    Decode {
        /// Input Ignition file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory for extracted files
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Encode extracted files back into an Ignition file
    Encode {
        /// Input decoded.ign file path
        #[arg(short, long)]
        input: PathBuf,

        /// Directory containing extracted files
        #[arg(short = 'd', long)]
        files_dir: PathBuf,

        /// Output Ignition file path
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { input, output } => {
            decode_ignition(&input, &output)?;
        }
        Commands::Encode {
            input,
            files_dir,
            output,
        } => {
            encode_ignition(&input, &files_dir, &output)?;
        }
    }

    Ok(())
}

fn decode_ignition(input_path: &Path, output_dir: &Path) -> Result<()> {
    // Read the input Ignition file
    let content = fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read input file: {}", input_path.display()))?;

    // Parse the Ignition config using ignition-config crate
    let (config, warnings) = Config::parse_str(&content)
        .with_context(|| "Failed to parse Ignition file")?;

    // Print warnings if any
    for warning in warnings {
        eprintln!("Warning: {}", warning);
    }

    // Create output directory
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

    // Work with the config based on version
    let (modified_json, file_counter) = match config {
        Config::V3_0(mut cfg) => decode_v3_config(&mut cfg, output_dir)?,
        Config::V3_1(mut cfg) => decode_v3_config(&mut cfg, output_dir)?,
        Config::V3_2(mut cfg) => decode_v3_config(&mut cfg, output_dir)?,
        Config::V3_3(mut cfg) => decode_v3_config(&mut cfg, output_dir)?,
        Config::V3_4(mut cfg) => decode_v3_config(&mut cfg, output_dir)?,
        Config::V3_5(mut cfg) => decode_v3_config(&mut cfg, output_dir)?,
        _ => anyhow::bail!("Unsupported Ignition config version"),
    };

    // Write the modified Ignition file
    let decoded_path = output_dir.join("decoded.ign");
    fs::write(&decoded_path, modified_json)
        .with_context(|| format!("Failed to write decoded.ign: {}", decoded_path.display()))?;

    println!(
        "\nDecoding complete! Extracted {} file(s) to {}",
        file_counter,
        output_dir.display()
    );
    println!("Modified Ignition file saved as: {}", decoded_path.display());

    Ok(())
}

// Generic function to handle all v3.x configs (they all have the same structure for our purposes)
fn decode_v3_config<T>(config: &mut T, output_dir: &Path) -> Result<(String, usize)>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    // Serialize to JSON value for manipulation
    let mut json_value: serde_json::Value = serde_json::to_value(&config)
        .with_context(|| "Failed to serialize config")?;

    let mut file_counter = 0;

    // Process storage.files
    if let Some(storage) = json_value.get_mut("storage") {
        if let Some(files) = storage.get_mut("files") {
            if let Some(files_array) = files.as_array_mut() {
                for file_obj in files_array.iter_mut() {
                    if let Some(contents) = file_obj.get_mut("contents") {
                        if let Some(source) = contents.get_mut("source") {
                            if let Some(source_str) = source.as_str() {
                                // Check if it's a data URL
                                if source_str.starts_with("data:") {
                                    file_counter += 1;
                                    let extracted_filename = format!("file_{:03}", file_counter);

                                    // Extract and decode the content
                                    let decoded_content = decode_data_url(source_str)?;

                                    // Save to file
                                    let extracted_path = output_dir.join(&extracted_filename);
                                    fs::write(&extracted_path, decoded_content).with_context(
                                        || {
                                            format!(
                                                "Failed to write extracted file: {}",
                                                extracted_path.display()
                                            )
                                        },
                                    )?;

                                    // Replace with placeholder
                                    *source =
                                        serde_json::Value::String(format!("file://./{}",  extracted_filename));

                                    println!("Extracted: {}", extracted_filename);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let pretty_json = serde_json::to_string_pretty(&json_value)
        .with_context(|| "Failed to serialize modified config")?;

    Ok((pretty_json, file_counter))
}

fn decode_data_url(data_url: &str) -> Result<Vec<u8>> {
    // Data URL format: data:[<mediatype>][;base64],<data>
    // Example: data:;base64,SGVsbG8gV29ybGQh
    // Example: data:text/plain;base64,SGVsbG8gV29ybGQh

    if !data_url.starts_with("data:") {
        anyhow::bail!("Invalid data URL: does not start with 'data:'");
    }

    // Find the comma that separates metadata from data
    let comma_pos = data_url
        .find(',')
        .ok_or_else(|| anyhow::anyhow!("Invalid data URL: missing comma separator"))?;

    let metadata = &data_url[5..comma_pos]; // Skip "data:"
    let data = &data_url[comma_pos + 1..];

    // Check if it's base64 encoded
    if metadata.ends_with(";base64") || metadata == ";base64" {
        // Decode base64
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(data)
            .with_context(|| "Failed to decode base64 content")?;
        Ok(decoded)
    } else {
        // Not base64, return as-is (URL-encoded data)
        Ok(data.as_bytes().to_vec())
    }
}

fn encode_ignition(decoded_path: &Path, files_dir: &Path, output_path: &Path) -> Result<()> {
    // Read the decoded Ignition file
    let content = fs::read_to_string(decoded_path)
        .with_context(|| format!("Failed to read decoded file: {}", decoded_path.display()))?;

    // Parse the Ignition config
    let (config, warnings) = Config::parse_str(&content)
        .with_context(|| "Failed to parse decoded Ignition file")?;

    // Print warnings if any
    for warning in warnings {
        eprintln!("Warning: {}", warning);
    }

    // Work with the config based on version
    let (modified_json, file_counter) = match config {
        Config::V3_0(cfg) => encode_v3_config(&cfg, files_dir)?,
        Config::V3_1(cfg) => encode_v3_config(&cfg, files_dir)?,
        Config::V3_2(cfg) => encode_v3_config(&cfg, files_dir)?,
        Config::V3_3(cfg) => encode_v3_config(&cfg, files_dir)?,
        Config::V3_4(cfg) => encode_v3_config(&cfg, files_dir)?,
        Config::V3_5(cfg) => encode_v3_config(&cfg, files_dir)?,
        _ => anyhow::bail!("Unsupported Ignition config version"),
    };

    // Write the encoded Ignition file
    fs::write(output_path, modified_json)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    println!(
        "\nEncoding complete! Encoded {} file(s) into {}",
        file_counter,
        output_path.display()
    );

    Ok(())
}

fn encode_v3_config<T>(config: &T, files_dir: &Path) -> Result<(String, usize)>
where
    T: serde::Serialize,
{
    // Serialize to JSON value for manipulation
    let mut json_value: serde_json::Value = serde_json::to_value(config)
        .with_context(|| "Failed to serialize config")?;

    let mut file_counter = 0;

    // Process storage.files
    if let Some(storage) = json_value.get_mut("storage") {
        if let Some(files) = storage.get_mut("files") {
            if let Some(files_array) = files.as_array_mut() {
                for file_obj in files_array.iter_mut() {
                    if let Some(contents) = file_obj.get_mut("contents") {
                        if let Some(source) = contents.get_mut("source") {
                            if let Some(source_str) = source.as_str() {
                                // Check if it's a file:// reference
                                if source_str.starts_with("file://") {
                                    let filename = source_str.trim_start_matches("file://./").to_string();

                                    // Read the file
                                    let file_path = files_dir.join(&filename);
                                    let file_content = fs::read(&file_path).with_context(|| {
                                        format!("Failed to read file: {}", file_path.display())
                                    })?;

                                    // Encode as base64
                                    use base64::Engine;
                                    let encoded = base64::engine::general_purpose::STANDARD
                                        .encode(&file_content);

                                    // Create data URL
                                    let data_url = format!("data:;base64,{}", encoded);

                                    // Replace the source
                                    *source = serde_json::Value::String(data_url);

                                    file_counter += 1;
                                    println!("Encoded: {}", filename);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let pretty_json = serde_json::to_string_pretty(&json_value)
        .with_context(|| "Failed to serialize encoded config")?;

    Ok((pretty_json, file_counter))
}
