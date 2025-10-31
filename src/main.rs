use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ignition_config::Config;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// Action to take when the target already exists
#[derive(clap::ValueEnum, Clone, Debug)]
enum Action {
    /// Create new target (fail if it already exists) - default
    New,
    /// Add to or update existing target (merge/overwrite files)
    Add,
    /// Replace entire target (remove existing first)
    Replace,
}

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

        /// Action to take with the target directory
        #[arg(long, default_value = "new")]
        action: Action,
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

        /// Action to take with the target file
        #[arg(long, default_value = "new")]
        action: Action,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Disassemble {
            ignition_file,
            target_dir,
            action,
        } => {
            disassemble_ignition(&ignition_file, &target_dir, action)?;
        }
        Commands::Assemble {
            target_file,
            ignition_dir,
            compact,
            default,
            action,
        } => {
            assemble_ignition(&target_file, &ignition_dir, compact, default, action)?;
        }
    }

    Ok(())
}

fn disassemble_ignition(input_path: &Path, output_dir: &Path, action: Action) -> Result<()> {
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

    // Handle target directory based on action
    if output_dir.exists() {
        match action {
            Action::New => {
                anyhow::bail!(
                    "Target directory already exists: {}. Use --action replace to overwrite or --action add to merge.",
                    output_dir.display()
                );
            }
            Action::Add => {
                // Directory exists, we'll add/overwrite files within it
                println!("Adding to existing directory: {}", output_dir.display());
            }
            Action::Replace => {
                fs::remove_dir_all(output_dir).with_context(|| {
                    format!(
                        "Failed to remove existing target directory: {}",
                        output_dir.display()
                    )
                })?;
                println!("Replaced existing directory: {}", output_dir.display());
            }
        }
    } else {
        // Directory doesn't exist, all actions will create it
        println!("Creating new directory: {}", output_dir.display());
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

    find_and_replace_source_with_path_update(&mut json_value, "", output_dir, &mut file_counter)?;

    let pretty_json = serde_json::to_string_pretty(&json_value)
        .with_context(|| "Failed to serialize modified config")?;

    Ok((pretty_json, file_counter))
}

fn find_and_replace_source<F>(value: &mut serde_json::Value, path: &str, func: &mut F) -> Result<()>
where
    F: FnMut(&str, &str, bool, usize) -> Result<String>,
{
    match value {
        serde_json::Value::Object(map) => {
            let mut new_path = path.to_string();
            if let Some(p) = map.get("path").and_then(|v| v.as_str()) {
                new_path = p.to_string();
            }

            // Check if this object has both a path and array fields with sources
            let has_path = map.contains_key("path");
            let mut found_array_with_sources = false;

            if has_path {
                // Look for array fields that contain objects with sources
                for (_key, val) in map.iter() {
                    if let serde_json::Value::Array(arr) = val {
                        // Check if any element in the array has a source
                        if arr.iter().any(|item| {
                            if let serde_json::Value::Object(obj) = item {
                                obj.contains_key("source")
                            } else {
                                false
                            }
                        }) {
                            found_array_with_sources = true;
                            break;
                        }
                    }
                }
            }

            for (key, val) in map.iter_mut() {
                if key == "source" {
                    if let Some(s) = val.as_str() {
                        *val = serde_json::Value::String(func(&new_path, s, false, 0)?);
                    }
                } else if found_array_with_sources && key != "path" {
                    // This might be an array field with sources
                    if let serde_json::Value::Array(arr) = val {
                        // Check if this array contains objects with sources
                        let has_sources = arr.iter().any(|item| {
                            if let serde_json::Value::Object(obj) = item {
                                obj.contains_key("source")
                            } else {
                                false
                            }
                        });

                        if has_sources {
                            // Process as array of sources
                            for (index, item) in arr.iter_mut().enumerate() {
                                process_array_item_sources(item, &new_path, index, func)?;
                            }
                        } else {
                            // Regular array processing
                            find_and_replace_source(val, &new_path, func)?;
                        }
                    } else {
                        find_and_replace_source(val, &new_path, func)?;
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

fn process_array_item_sources<F>(
    item: &mut serde_json::Value,
    path: &str,
    index: usize,
    func: &mut F,
) -> Result<()>
where
    F: FnMut(&str, &str, bool, usize) -> Result<String>,
{
    if let serde_json::Value::Object(map) = item {
        for (key, val) in map.iter_mut() {
            if key == "source" {
                if let Some(s) = val.as_str() {
                    *val = serde_json::Value::String(func(path, s, true, index)?);
                }
            } else {
                // Recursively process nested structures
                process_nested_sources(val, path, index, func)?;
            }
        }
    }
    Ok(())
}

fn process_nested_sources<F>(
    value: &mut serde_json::Value,
    path: &str,
    index: usize,
    func: &mut F,
) -> Result<()>
where
    F: FnMut(&str, &str, bool, usize) -> Result<String>,
{
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if key == "source" {
                    if let Some(s) = val.as_str() {
                        *val = serde_json::Value::String(func(path, s, true, index)?);
                    }
                } else {
                    process_nested_sources(val, path, index, func)?;
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                process_nested_sources(val, path, index, func)?;
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
    action: Action,
) -> Result<()> {
    // Handle target file based on action
    if target_file.exists() {
        match action {
            Action::New => {
                anyhow::bail!(
                    "Target file already exists: {}. Use --action replace or --action add to overwrite.",
                    target_file.display()
                );
            }
            Action::Add | Action::Replace => {
                fs::remove_file(target_file).with_context(|| {
                    format!(
                        "Failed to remove existing target file: {}",
                        target_file.display()
                    )
                })?;
                println!("Overwriting existing file: {}", target_file.display());
            }
        }
    } else {
        // File doesn't exist, all actions will create it
        println!("Creating new file: {}", target_file.display());
    }

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

    find_and_replace_source(
        &mut json_value,
        "",
        &mut |_path, source_str, _is_array, _array_index| {
            if source_str.contains(";base64-placeholder,") {
                // Extract media type and file path from the placeholder
                let after_data = source_str.trim_start_matches("data:");
                let parts: Vec<&str> = after_data.splitn(2, ";base64-placeholder,").collect();
                let media_type = parts[0];
                let file_path = if parts.len() > 1 && !parts[1].is_empty() {
                    parts[1]
                } else {
                    return Err(anyhow::anyhow!(
                        "Invalid placeholder format: missing file path"
                    ));
                };

                let in_path = files_dir.join(file_path);

                let file_content = fs::read(&in_path)?;
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(&file_content);
                file_counter += 1;
                Ok(format!("data:{};base64,{}", media_type, encoded))
            } else {
                Ok(source_str.to_string())
            }
        },
    )?;

    let pretty_json = serde_json::to_string_pretty(&json_value)
        .with_context(|| "Failed to serialize encoded config")?;

    Ok((pretty_json, file_counter))
}

fn find_and_replace_source_with_path_update(
    value: &mut serde_json::Value,
    path: &str,
    output_dir: &Path,
    file_counter: &mut usize,
) -> Result<()> {
    match value {
        serde_json::Value::Object(map) => {
            let mut new_path = path.to_string();
            if let Some(p) = map.get("path").and_then(|v| v.as_str()) {
                new_path = p.to_string();
            }

            // Check if this object has both a path and array fields with sources
            let has_path = map.contains_key("path");
            let mut found_array_with_sources = false;

            if has_path {
                // Look for array fields that contain objects with sources
                for (_key, val) in map.iter() {
                    if let serde_json::Value::Array(arr) = val {
                        // Check if any element in the array has a source
                        if arr.iter().any(|item| {
                            if let serde_json::Value::Object(obj) = item {
                                obj.contains_key("source")
                            } else {
                                false
                            }
                        }) {
                            found_array_with_sources = true;
                            break;
                        }
                    }
                }
            }

            for (key, val) in map.iter_mut() {
                if key == "source" {
                    if let Some(source_str) = val.as_str() {
                        if source_str.starts_with("data:") {
                            let url = data_url::DataUrl::process(source_str).map_err(|e| {
                                anyhow::anyhow!("Failed to parse data URL: {:?}", e)
                            })?;
                            let (decoded_content, _) = url.decode_to_vec().unwrap();
                            let media_type = url.mime_type().to_string();

                            let relative_path = new_path.trim_start_matches("/");

                            // Handle empty path by providing a default filename based on content type
                            let effective_path = if relative_path.is_empty() {
                                // Generate a filename based on the media type
                                let extension = match media_type.as_str() {
                                    "text/plain" => "data",
                                    "application/json" => "json",
                                    "application/yaml" => "yaml",
                                    "text/yaml" => "yaml",
                                    "application/x-yaml" => "yaml",
                                    "text/x-yaml" => "yaml",
                                    "application/xml" => "xml",
                                    "text/xml" => "xml",
                                    "text/html" => "html",
                                    "application/javascript" => "js",
                                    "text/css" => "css",
                                    _ => "data",
                                };
                                format!("extracted_file_{}.{}", *file_counter, extension)
                            } else {
                                relative_path.to_string()
                            };

                            // Create the output file
                            let file_path = output_dir.join(&effective_path);
                            if let Some(parent) = file_path.parent() {
                                fs::create_dir_all(parent)?;
                            }

                            fs::write(&file_path, decoded_content)?;

                            // Update the source field to placeholder with relative file path
                            *val = serde_json::Value::String(format!(
                                "data:{};base64-placeholder,{}",
                                media_type, effective_path
                            ));

                            *file_counter += 1;
                        }
                    }
                } else if found_array_with_sources && key != "path" {
                    // This might be an array field with sources
                    if let serde_json::Value::Array(arr) = val {
                        // Check if this array contains objects with sources
                        let has_sources = arr.iter().any(|item| {
                            if let serde_json::Value::Object(obj) = item {
                                obj.contains_key("source")
                            } else {
                                false
                            }
                        });

                        if has_sources {
                            // Process as array of sources
                            for (index, item) in arr.iter_mut().enumerate() {
                                process_array_item_sources_with_path_update(
                                    item,
                                    &new_path,
                                    index,
                                    output_dir,
                                    file_counter,
                                )?;
                            }
                        } else {
                            // Recursively process nested structures
                            find_and_replace_source_with_path_update(
                                val,
                                &new_path,
                                output_dir,
                                file_counter,
                            )?;
                        }
                    } else {
                        // Recursively process other nested structures
                        find_and_replace_source_with_path_update(
                            val,
                            &new_path,
                            output_dir,
                            file_counter,
                        )?;
                    }
                } else {
                    // Recursively process nested objects and arrays
                    find_and_replace_source_with_path_update(
                        val,
                        &new_path,
                        output_dir,
                        file_counter,
                    )?;
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                find_and_replace_source_with_path_update(val, path, output_dir, file_counter)?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn process_array_item_sources_with_path_update(
    item: &mut serde_json::Value,
    path: &str,
    index: usize,
    output_dir: &Path,
    file_counter: &mut usize,
) -> Result<()> {
    if let serde_json::Value::Object(map) = item {
        for (key, val) in map.iter_mut() {
            if key == "source" {
                if let Some(source_str) = val.as_str() {
                    if source_str.starts_with("data:") {
                        let url = data_url::DataUrl::process(source_str)
                            .map_err(|e| anyhow::anyhow!("Failed to parse data URL: {:?}", e))?;
                        let (decoded_content, _) = url.decode_to_vec().unwrap();
                        let media_type = url.mime_type().to_string();

                        let relative_path = path.trim_start_matches("/");

                        // Handle empty path by providing a default filename
                        let effective_path = if relative_path.is_empty() {
                            let extension = match media_type.as_str() {
                                "text/plain" => "data",
                                "application/json" => "json",
                                "application/yaml" => "yaml",
                                "text/yaml" => "yaml",
                                "application/x-yaml" => "yaml",
                                "text/x-yaml" => "yaml",
                                "application/xml" => "xml",
                                "text/xml" => "xml",
                                "text/html" => "html",
                                "application/javascript" => "js",
                                "text/css" => "css",
                                _ => "data",
                            };
                            format!("extracted_file_{}.{}", *file_counter, extension)
                        } else {
                            relative_path.to_string()
                        };

                        // Create directory and indexed file for array items
                        let dir_path = output_dir.join(&effective_path);
                        fs::create_dir_all(&dir_path)?;
                        let file_path = dir_path.join(index.to_string());

                        fs::write(&file_path, decoded_content)?;

                        // Update the source field to placeholder with array path
                        let array_file_path = format!("{}/{}", effective_path, index);
                        *val = serde_json::Value::String(format!(
                            "data:{};base64-placeholder,{}",
                            media_type, array_file_path
                        ));

                        *file_counter += 1;
                    }
                }
            } else {
                // Recursively process nested structures
                process_nested_sources_with_path_update(
                    val,
                    path,
                    index,
                    output_dir,
                    file_counter,
                )?;
            }
        }
    }
    Ok(())
}

fn process_nested_sources_with_path_update(
    value: &mut serde_json::Value,
    path: &str,
    index: usize,
    output_dir: &Path,
    file_counter: &mut usize,
) -> Result<()> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if key == "source" {
                    if let Some(source_str) = val.as_str() {
                        if source_str.starts_with("data:") {
                            let url = data_url::DataUrl::process(source_str).map_err(|e| {
                                anyhow::anyhow!("Failed to parse data URL: {:?}", e)
                            })?;
                            let (decoded_content, _) = url.decode_to_vec().unwrap();
                            let media_type = url.mime_type().to_string();

                            let relative_path = path.trim_start_matches("/");

                            let effective_path = if relative_path.is_empty() {
                                let extension = match media_type.as_str() {
                                    "text/plain" => "data",
                                    "application/json" => "json",
                                    "application/yaml" => "yaml",
                                    "text/yaml" => "yaml",
                                    "application/x-yaml" => "yaml",
                                    "text/x-yaml" => "yaml",
                                    "application/xml" => "xml",
                                    "text/xml" => "xml",
                                    "text/html" => "html",
                                    "application/javascript" => "js",
                                    "text/css" => "css",
                                    _ => "data",
                                };
                                format!("extracted_file_{}.{}", *file_counter, extension)
                            } else {
                                relative_path.to_string()
                            };

                            let dir_path = output_dir.join(&effective_path);
                            fs::create_dir_all(&dir_path)?;
                            let file_path = dir_path.join(index.to_string());

                            fs::write(&file_path, decoded_content)?;

                            let array_file_path = format!("{}/{}", effective_path, index);
                            *val = serde_json::Value::String(format!(
                                "data:{};base64-placeholder,{}",
                                media_type, array_file_path
                            ));

                            *file_counter += 1;
                        }
                    }
                } else {
                    process_nested_sources_with_path_update(
                        val,
                        path,
                        index,
                        output_dir,
                        file_counter,
                    )?;
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                process_nested_sources_with_path_update(
                    val,
                    path,
                    index,
                    output_dir,
                    file_counter,
                )?;
            }
        }
        _ => (),
    }
    Ok(())
}
