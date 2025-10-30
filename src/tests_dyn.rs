//! Dynamic tests for fcos-ignition-coder
//!
//! This module contains tests that run against all ignition files found in the ./examples/ directory.
//! Unlike the static tests in tests.rs which use hardcoded test data, these tests dynamically
//! discover example ignition files and perform round-trip testing on them.
//!
//! The main test functions are:
//! - `test_roundtrip_all_examples`: Tests round-trip (disassemble -> assemble) for all example files
//! - `test_disassemble_all_examples`: Tests disassembly of all example files
//! - `test_assemble_all_examples`: Tests assembly of all example files
//! - `test_roundtrip_example_ign`: Specific test for examples/example.ign
//! - `test_roundtrip_bootstrap_ign`: Specific test for examples/bootstrap.ign
//!
//! Round-trip testing verifies that:
//! 1. An ignition file can be disassembled into its component parts
//! 2. The disassembled parts can be reassembled back into a valid ignition file
//! 3. The file contents extracted from the reassembled ignition file are identical to the original

#[cfg(test)]
mod tests {
    use crate::{assemble_ignition, disassemble_ignition};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn get_example_ignition_files() -> Vec<std::path::PathBuf> {
        let examples_dir = Path::new("examples");
        let mut ignition_files = Vec::new();

        if examples_dir.exists() && examples_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(examples_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("ign") {
                        ignition_files.push(path);
                    }
                }
            }
        }

        ignition_files.sort();
        ignition_files
    }

    fn test_roundtrip_for_file(ignition_file: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let decoded_dir = temp_dir.path().join("decoded");
        let output_path = temp_dir.path().join("output.ign");

        // Read the original ignition file
        let original_content = fs::read_to_string(ignition_file)?;

        // Create a temporary input file from the example
        let input_path = temp_dir.path().join("input.ign");
        fs::write(&input_path, &original_content)?;

        // Disassemble the ignition file
        disassemble_ignition(&input_path, &decoded_dir)?;

        // Store the extracted file contents for comparison
        let mut original_file_contents = std::collections::HashMap::new();
        for entry in fs::read_dir(&decoded_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.file_name().unwrap() != "decoded.ign" {
                let relative_path = path.strip_prefix(&decoded_dir)?;
                let content = fs::read(&path)?;
                original_file_contents.insert(relative_path.to_path_buf(), content);
            }
        }

        // Assemble it back
        assemble_ignition(&output_path, &decoded_dir, false, true)?;

        // Disassemble the output again to compare extracted files
        let decoded_dir2 = temp_dir.path().join("decoded2");
        disassemble_ignition(&output_path, &decoded_dir2)?;

        // Compare the extracted file contents
        for entry in fs::read_dir(&decoded_dir2)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.file_name().unwrap() != "decoded.ign" {
                let relative_path = path.strip_prefix(&decoded_dir2)?;
                let content = fs::read(&path)?;

                match original_file_contents.get(relative_path) {
                    Some(original_content) => {
                        if &content != original_content {
                            return Err(format!(
                                "File content mismatch for {} in {}. Round-trip changed file content.",
                                relative_path.display(),
                                ignition_file.display()
                            ).into());
                        }
                    }
                    None => {
                        return Err(format!(
                            "Extra file {} found after round-trip for {}",
                            relative_path.display(),
                            ignition_file.display()
                        )
                        .into());
                    }
                }
            }
        }

        // Check that we have the same number of files
        let output_file_count = fs::read_dir(&decoded_dir2)?
            .filter(|entry| {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    path.is_file() && path.file_name().unwrap() != "decoded.ign"
                } else {
                    false
                }
            })
            .count();

        if output_file_count != original_file_contents.len() {
            return Err(format!(
                "File count mismatch for {}. Original: {}, Output: {}",
                ignition_file.display(),
                original_file_contents.len(),
                output_file_count
            )
            .into());
        }

        Ok(())
    }

    #[test]
    fn test_roundtrip_all_examples() {
        let ignition_files = get_example_ignition_files();

        if ignition_files.is_empty() {
            panic!("No .ign files found in examples directory");
        }

        println!("Found {} ignition files in examples/", ignition_files.len());

        let mut failed_files = Vec::new();
        let mut passed_count = 0;

        for ignition_file in &ignition_files {
            println!("Testing round-trip for: {}", ignition_file.display());

            match test_roundtrip_for_file(ignition_file) {
                Ok(()) => {
                    println!("✓ Round-trip test passed for {}", ignition_file.display());
                    passed_count += 1;
                }
                Err(e) => {
                    println!(
                        "✗ Round-trip test failed for {}: {}",
                        ignition_file.display(),
                        e
                    );
                    failed_files.push((ignition_file.clone(), e.to_string()));
                }
            }
        }

        println!("\nTest Summary:");
        println!("Passed: {}/{}", passed_count, ignition_files.len());
        println!("Failed: {}/{}", failed_files.len(), ignition_files.len());

        if !failed_files.is_empty() {
            println!("\nFailed files:");
            for (file, error) in &failed_files {
                println!("  {}: {}", file.display(), error);
            }
            panic!("{} round-trip tests failed", failed_files.len());
        }
    }

    #[test]
    fn test_roundtrip_example_ign() {
        let example_file = Path::new("examples/example.ign");
        if example_file.exists() {
            test_roundtrip_for_file(example_file)
                .expect("Round-trip test should pass for examples/example.ign");
        } else {
            panic!("examples/example.ign not found");
        }
    }

    #[test]
    fn test_roundtrip_bootstrap_ign() {
        let bootstrap_file = Path::new("examples/bootstrap.ign");
        if bootstrap_file.exists() {
            test_roundtrip_for_file(bootstrap_file)
                .expect("Round-trip test should pass for examples/bootstrap.ign");
        } else {
            println!("Skipping bootstrap.ign test - file not found");
        }
    }

    #[test]
    fn test_disassemble_all_examples() {
        let ignition_files = get_example_ignition_files();

        if ignition_files.is_empty() {
            panic!("No .ign files found in examples directory");
        }

        for ignition_file in &ignition_files {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let decoded_dir = temp_dir.path().join("decoded");

            println!("Testing disassemble for: {}", ignition_file.display());

            disassemble_ignition(ignition_file, &decoded_dir).expect(&format!(
                "Failed to disassemble {}",
                ignition_file.display()
            ));

            // Verify that decoded.ign was created
            assert!(
                decoded_dir.join("decoded.ign").exists(),
                "decoded.ign should be created for {}",
                ignition_file.display()
            );

            // Verify that the decoded.ign is valid JSON
            let decoded_content = fs::read_to_string(decoded_dir.join("decoded.ign"))
                .expect("Failed to read decoded.ign");

            serde_json::from_str::<serde_json::Value>(&decoded_content).expect(&format!(
                "decoded.ign should be valid JSON for {}",
                ignition_file.display()
            ));

            println!("✓ Disassemble test passed for {}", ignition_file.display());
        }
    }

    #[test]
    fn test_empty_examples_directory() {
        // Test behavior when no .ign files are found
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let fake_examples_dir = temp_dir.path().join("fake_examples");
        fs::create_dir(&fake_examples_dir).expect("Failed to create fake examples dir");

        // Temporarily change to the temp directory to test the case where examples/ exists but is empty
        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(temp_dir.path()).expect("Failed to change to temp dir");

        // Create an empty examples directory
        fs::create_dir("examples").expect("Failed to create examples dir");

        let ignition_files = get_example_ignition_files();
        assert!(
            ignition_files.is_empty(),
            "Should find no ignition files in empty directory"
        );

        // Restore original directory
        std::env::set_current_dir(original_dir).expect("Failed to restore original dir");
    }

    #[test]
    fn test_missing_examples_directory() {
        // Test behavior when examples directory doesn't exist
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(temp_dir.path()).expect("Failed to change to temp dir");

        // Don't create examples directory - it shouldn't exist
        let ignition_files = get_example_ignition_files();
        assert!(
            ignition_files.is_empty(),
            "Should find no ignition files when directory doesn't exist"
        );

        // Restore original directory
        std::env::set_current_dir(original_dir).expect("Failed to restore original dir");
    }

    #[test]
    fn test_assemble_all_examples() {
        let ignition_files = get_example_ignition_files();

        if ignition_files.is_empty() {
            panic!("No .ign files found in examples directory");
        }

        for ignition_file in &ignition_files {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let decoded_dir = temp_dir.path().join("decoded");
            let output_path = temp_dir.path().join("assembled.ign");

            println!("Testing assemble for: {}", ignition_file.display());

            // First disassemble
            disassemble_ignition(ignition_file, &decoded_dir).expect(&format!(
                "Failed to disassemble {}",
                ignition_file.display()
            ));

            // Then assemble
            assemble_ignition(&output_path, &decoded_dir, false, true)
                .expect(&format!("Failed to assemble {}", ignition_file.display()));

            // Verify output file exists and is valid JSON
            assert!(
                output_path.exists(),
                "Assembled file should exist for {}",
                ignition_file.display()
            );

            let assembled_content =
                fs::read_to_string(&output_path).expect("Failed to read assembled file");

            serde_json::from_str::<serde_json::Value>(&assembled_content).expect(&format!(
                "Assembled file should be valid JSON for {}",
                ignition_file.display()
            ));

            println!("✓ Assemble test passed for {}", ignition_file.display());
        }
    }
}
