use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ignition_config::Config;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_dyn;

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
    #[command(aliases = ["decode", "d", "div"])]
    Disassemble {
        /// The ignition file to decode
        ignition_file: PathBuf,

        /// The directory to place the decoded files in
        target_dir: PathBuf,
    },
    /// Encode extracted files back into an Ignition file
    #[command(aliases = ["encode", "a", "prod"])]
    Assemble {
        /// The file to write the encoded ignition to
        target_file: PathBuf,

        /// The directory containing the ignition file and file contents
        ignition_dir: PathBuf,

        /// Serialize the output in a compact format
        #[arg(long)]
        compact: bool,

        /// Suppress fields that have default values
        #[arg(long)]
        default: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Disassemble {
            ignition_file,
            target_dir,
        } => {
            disassemble_ignition(&ignition_file, &target_dir)?;
        }
        Commands::Assemble {
            target_file,
            ignition_dir,
            compact,
            default,
        } => {
            assemble_ignition(&target_file, &ignition_dir, compact, default)?;
        }
    }

    Ok(())
}

fn disassemble_ignition(input_path: &Path, output_dir: &Path) -> Result<()> {
    // Read the input Ignition file
    let content = fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read input file: {}", input_path.display()))?;

    // Parse the Ignition config using ignition-config crate
    let (mut config, warnings) =
        Config::parse_str(&content).with_context(|| "Failed to parse Ignition file")?;

    // Print warnings if any
    for warning in warnings {
        eprintln!("Warning: {}", warning);
    }

    // Create output directory
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    // Work with the config based on version
    let (modified_json, file_counter) = match &mut config {
        Config::V3_0(cfg) => disassemble_v3_config(cfg, output_dir)?,
        Config::V3_1(cfg) => disassemble_v3_config(cfg, output_dir)?,
        Config::V3_2(cfg) => disassemble_v3_config(cfg, output_dir)?,
        Config::V3_3(cfg) => disassemble_v3_config(cfg, output_dir)?,
        Config::V3_4(cfg) => disassemble_v3_config(cfg, output_dir)?,
        Config::V3_5(cfg) => disassemble_v3_config(cfg, output_dir)?,
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
    println!(
        "Modified Ignition file saved as: {}",
        decoded_path.display()
    );

    Ok(())
}

// Generic function to handle all v3.x configs (they all have the same structure for our purposes)
fn disassemble_v3_config<T>(config: &mut T, output_dir: &Path) -> Result<(String, usize)>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    // Serialize to JSON value for manipulation
    let mut json_value: serde_json::Value =
        serde_json::to_value(&config).with_context(|| "Failed to serialize config")?;

    let mut file_counter = 0;

    find_and_replace_source(&mut json_value, "", &mut |path, source_str| {
        if source_str.starts_with("data:") {
            let url = data_url::DataUrl::process(source_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse data URL: {:?}", e))?;
            let (decoded_content, _) = url.decode_to_vec().unwrap();
            let media_type = url.mime_type().to_string();

            let relative_path = path.trim_start_matches("/");
            let out_path = output_dir.join(relative_path);
            fs::create_dir_all(out_path.parent().unwrap())?;
            fs::write(&out_path, decoded_content)?;

            file_counter += 1;
            Ok(format!("data:{};base64-placeholder,", media_type))
        } else {
            Ok(source_str.to_string())
        }
    })?;

    let pretty_json = serde_json::to_string_pretty(&json_value)
        .with_context(|| "Failed to serialize modified config")?;

    Ok((pretty_json, file_counter))
}

fn find_and_replace_source<F>(value: &mut serde_json::Value, path: &str, func: &mut F) -> Result<()>
where
    F: FnMut(&str, &str) -> Result<String>,
{
    match value {
        serde_json::Value::Object(map) => {
            let mut new_path = path.to_string();
            if let Some(p) = map.get("path").and_then(|v| v.as_str()) {
                new_path = p.to_string();
            }

            for (key, val) in map.iter_mut() {
                if key == "source" {
                    if let Some(s) = val.as_str() {
                        *val = serde_json::Value::String(func(&new_path, s)?);
                    }
                } else {
                    find_and_replace_source(val, &new_path, func)?;
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                find_and_replace_source(val, path, func)?;
            }
        }
        _ => (),
    }
    Ok(())
}

fn remove_default_values(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.retain(|_, v| !is_default(v));
            for (_, v) in map.iter_mut() {
                remove_default_values(v);
            }
        }
        serde_json::Value::Array(arr) => {
            arr.retain(|v| !is_default(v));
            for v in arr.iter_mut() {
                remove_default_values(v);
            }
        }
        _ => (),
    }
}

fn is_default(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        serde_json::Value::Array(arr) => arr.is_empty(),
        serde_json::Value::Object(map) => map.is_empty(),
        serde_json::Value::Bool(b) => !b,
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(1) == 0,
    }
}

fn assemble_ignition(
    target_file: &Path,
    ignition_dir: &Path,
    compact: bool,
    default: bool,
) -> Result<()> {
    // Find the .ign file in the ignition_dir
    let mut ignition_file = None;
    for entry in fs::read_dir(ignition_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("ign") {
            ignition_file = Some(path);
            break;
        }
    }

    let ignition_file = ignition_file.ok_or_else(|| {
        anyhow::anyhow!(
            "No .ign file found in ignition_dir: {}",
            ignition_dir.display()
        )
    })?;

    // Read the decoded Ignition file
    let content = fs::read_to_string(&ignition_file)
        .with_context(|| format!("Failed to read decoded file: {}", ignition_file.display()))?;

    // Parse the Ignition config
    let (config, warnings) =
        Config::parse_str(&content).with_context(|| "Failed to parse decoded Ignition file")?;

    // Print warnings if any
    for warning in warnings {
        eprintln!("Warning: {}", warning);
    }

    // Work with the config based on version
    let (mut modified_json, file_counter) = match config {
        Config::V3_0(cfg) => assemble_v3_config(&cfg, ignition_dir)?,
        Config::V3_1(cfg) => assemble_v3_config(&cfg, ignition_dir)?,
        Config::V3_2(cfg) => assemble_v3_config(&cfg, ignition_dir)?,
        Config::V3_3(cfg) => assemble_v3_config(&cfg, ignition_dir)?,
        Config::V3_4(cfg) => assemble_v3_config(&cfg, ignition_dir)?,
        Config::V3_5(cfg) => assemble_v3_config(&cfg, ignition_dir)?,
        _ => anyhow::bail!("Unsupported Ignition config version"),
    };

    if default {
        let mut json_value: serde_json::Value = serde_json::from_str(&modified_json)?;
        remove_default_values(&mut json_value);
        modified_json = if compact {
            serde_json::to_string(&json_value)?
        } else {
            serde_json::to_string_pretty(&json_value)?
        };
    } else {
        if compact {
            let json_value: serde_json::Value = serde_json::from_str(&modified_json)?;
            modified_json = serde_json::to_string(&json_value)?;
        }
    }

    // Write the encoded Ignition file
    fs::write(target_file, modified_json)
        .with_context(|| format!("Failed to write output file: {}", target_file.display()))?;

    println!(
        "\nEncoding complete! Encoded {} file(s) into {}",
        file_counter,
        target_file.display()
    );

    Ok(())
}

fn assemble_v3_config<T>(config: &T, files_dir: &Path) -> Result<(String, usize)>
where
    T: serde::Serialize,
{
    // Serialize to JSON value for manipulation
    let mut json_value: serde_json::Value =
        serde_json::to_value(config).with_context(|| "Failed to serialize config")?;

    let mut file_counter = 0;

    find_and_replace_source(&mut json_value, "", &mut |path, source_str| {
        if source_str.contains(";base64-placeholder,") {
            let media_type = source_str
                .trim_start_matches("data:")
                .trim_end_matches(";base64-placeholder,");
            let relative_path = path.trim_start_matches("/");
            let in_path = files_dir.join(relative_path);
            let file_content = fs::read(&in_path)?;
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&file_content);
            file_counter += 1;
            Ok(format!("data:{};base64,{}", media_type, encoded))
        } else {
            Ok(source_str.to_string())
        }
    })?;

    let pretty_json = serde_json::to_string_pretty(&json_value)
        .with_context(|| "Failed to serialize encoded config")?;

    Ok((pretty_json, file_counter))
}
