#[cfg(test)]
mod tests {
    use crate::{assemble_ignition, disassemble_ignition};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_disassemble() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("test.ign");
        let decoded_dir = temp_dir.path().join("decoded");

        let test_ignition = r#"{
  "ignition": {
    "version": "3.4.0"
  },
  "storage": {
    "files": [
      {
        "path": "/etc/test",
        "mode": 420,
        "contents": {
          "source": "data:;base64,dGVzdCBjb250ZW50"
        }
      }
    ]
  }
}"#;
        fs::write(&input_path, test_ignition).unwrap();

        disassemble_ignition(&input_path, &decoded_dir, false).unwrap();

        assert!(decoded_dir.join("decoded.ign").exists());
        assert!(decoded_dir.join("etc/test").exists());

        let decoded_content = fs::read_to_string(decoded_dir.join("etc/test")).unwrap();
        assert_eq!(decoded_content, "test content");

        let decoded_ign_content = fs::read_to_string(decoded_dir.join("decoded.ign")).unwrap();
        assert!(decoded_ign_content
            .contains("data:text/plain;charset=US-ASCII;base64-placeholder,etc/test"));
    }

    #[test]
    fn test_assemble() {
        let temp_dir = TempDir::new().unwrap();
        let ignition_dir = temp_dir.path().join("ignition");
        fs::create_dir(&ignition_dir).unwrap();
        let target_file = temp_dir.path().join("output.ign");

        let decoded_ign = r#"{
  "ignition": {
    "version": "3.4.0"
  },
  "storage": {
    "files": [
      {
        "path": "/etc/test",
        "mode": 420,
        "contents": {
          "source": "data:text/plain;charset=US-ASCII;base64-placeholder,etc/test"
        }
      }
    ]
  }
}"#;
        fs::write(ignition_dir.join("decoded.ign"), decoded_ign).unwrap();

        let file_content = "test content";
        let file_path = ignition_dir.join("etc/test");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(file_path, file_content).unwrap();

        assemble_ignition(&target_file, &ignition_dir, false, true, false).unwrap();

        assert!(target_file.exists());

        let output_content = fs::read_to_string(&target_file).unwrap();
        assert!(output_content.contains("data:text/plain;charset=US-ASCII;base64,dGVzdCBjb250ZW50"));
    }

    #[test]
    fn test_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("test.ign");
        let decoded_dir = temp_dir.path().join("decoded");
        let output_path = temp_dir.path().join("output.ign");

        let test_ignition = r#"{
  "ignition": {
    "version": "3.4.0"
  },
  "storage": {
    "files": [
      {
        "path": "/etc/test",
        "mode": 420,
        "contents": {
          "source": "data:text/plain;charset=US-ASCII;base64,dGVzdCBjb250ZW50"
        }
      }
    ]
  }
}"#;
        fs::write(&input_path, test_ignition).unwrap();

        disassemble_ignition(&input_path, &decoded_dir, false).unwrap();
        assemble_ignition(&output_path, &decoded_dir, false, true, false).unwrap();

        let input_json: serde_json::Value = serde_json::from_str(test_ignition).unwrap();
        let output_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(output_path).unwrap()).unwrap();

        assert_eq!(input_json, output_json);
    }

    #[test]
    fn test_array_sources() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("test_array.ign");
        let decoded_dir = temp_dir.path().join("decoded");

        let test_ignition = r#"{
  "ignition": {
    "version": "3.4.0"
  },
  "storage": {
    "files": [
      {
        "path": "/etc/test-single",
        "contents": {
          "source": "data:text/plain;charset=utf-8;base64,U2luZ2xlIGZpbGU="
        },
        "mode": 420
      },
      {
        "path": "/etc/test-array",
        "append": [
          {
            "source": "data:text/plain;charset=utf-8;base64,Rmlyc3QgZW50cnk="
          },
          {
            "source": "data:text/plain;charset=utf-8;base64,U2Vjb25kIGVudHJ5"
          }
        ],
        "mode": 644
      }
    ]
  }
}"#;
        fs::write(&input_path, test_ignition).unwrap();

        disassemble_ignition(&input_path, &decoded_dir, false).unwrap();

        // Check single file was created as a file
        assert!(decoded_dir.join("etc/test-single").exists());
        assert!(decoded_dir.join("etc/test-single").is_file());
        let single_content = fs::read_to_string(decoded_dir.join("etc/test-single")).unwrap();
        assert_eq!(single_content, "Single file");

        // Check array sources were created as directory with indexed files
        assert!(decoded_dir.join("etc/test-array").exists());
        assert!(decoded_dir.join("etc/test-array").is_dir());
        assert!(decoded_dir.join("etc/test-array/0").exists());
        assert!(decoded_dir.join("etc/test-array/1").exists());

        let first_content = fs::read_to_string(decoded_dir.join("etc/test-array/0")).unwrap();
        assert_eq!(first_content, "First entry");

        let second_content = fs::read_to_string(decoded_dir.join("etc/test-array/1")).unwrap();
        assert_eq!(second_content, "Second entry");
    }

    #[test]
    fn test_array_sources_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("test_array.ign");
        let decoded_dir = temp_dir.path().join("decoded");
        let output_path = temp_dir.path().join("output.ign");

        let test_ignition = r#"{
  "ignition": {
    "version": "3.4.0"
  },
  "storage": {
    "files": [
      {
        "path": "/etc/motd",
        "append": [
          {
            "source": "data:text/plain;charset=utf-8;base64,SGVsbG8gV29ybGQ="
          },
          {
            "source": "data:text/plain;charset=utf-8;base64,R29vZGJ5ZSBXb3JsZA=="
          }
        ],
        "mode": 420
      }
    ]
  }
}"#;
        fs::write(&input_path, test_ignition).unwrap();

        // Disassemble
        disassemble_ignition(&input_path, &decoded_dir, false).unwrap();

        // Verify array structure was created
        assert!(decoded_dir.join("etc/motd").is_dir());
        assert!(decoded_dir.join("etc/motd/0").exists());
        assert!(decoded_dir.join("etc/motd/1").exists());

        // Assemble back
        assemble_ignition(&output_path, &decoded_dir, false, true, false).unwrap();

        // Parse both JSON files to compare structure
        let input_json: serde_json::Value = serde_json::from_str(test_ignition).unwrap();
        let output_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(output_path).unwrap()).unwrap();

        // Extract the append arrays for comparison
        let input_append = &input_json["storage"]["files"][0]["append"];
        let output_append = &output_json["storage"]["files"][0]["append"];

        // Verify both have the same number of entries
        assert_eq!(
            input_append.as_array().unwrap().len(),
            output_append.as_array().unwrap().len()
        );

        // Verify the source content is preserved (though compression field might be added)
        for i in 0..2 {
            let input_source = input_append[i]["source"].as_str().unwrap();
            let output_source = output_append[i]["source"].as_str().unwrap();

            // Both should contain the same base64 content
            assert!(
                input_source.contains("SGVsbG8gV29ybGQ=")
                    || input_source.contains("R29vZGJ5ZSBXb3JsZA==")
            );
            assert!(
                output_source.contains("SGVsbG8gV29ybGQ=")
                    || output_source.contains("R29vZGJ5ZSBXb3JsZA==")
            );
        }
    }

    #[test]
    fn test_disassemble_replace_flag() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("test.ign");
        let decoded_dir = temp_dir.path().join("decoded");

        let test_ignition = r#"{
  "ignition": {
    "version": "3.4.0"
  },
  "storage": {
    "files": [
      {
        "path": "/etc/test",
        "mode": 420,
        "contents": {
          "source": "data:;base64,dGVzdCBjb250ZW50"
        }
      }
    ]
  }
}"#;
        fs::write(&input_path, test_ignition).unwrap();

        // Create existing target directory with some content
        fs::create_dir_all(&decoded_dir).unwrap();
        fs::write(decoded_dir.join("existing_file"), "old content").unwrap();

        // Without replace flag, should fail
        let result = disassemble_ignition(&input_path, &decoded_dir, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Target directory already exists"));

        // With replace flag, should succeed
        disassemble_ignition(&input_path, &decoded_dir, true).unwrap();

        // Verify old content was removed and new content exists
        assert!(!decoded_dir.join("existing_file").exists());
        assert!(decoded_dir.join("decoded.ign").exists());
        assert!(decoded_dir.join("etc/test").exists());
    }

    #[test]
    fn test_assemble_replace_flag() {
        let temp_dir = TempDir::new().unwrap();
        let ignition_dir = temp_dir.path().join("ignition");
        fs::create_dir(&ignition_dir).unwrap();
        let target_file = temp_dir.path().join("output.ign");

        let decoded_ign = r#"{
  "ignition": {
    "version": "3.4.0"
  },
  "storage": {
    "files": [
      {
        "path": "/etc/test",
        "mode": 420,
        "contents": {
          "source": "data:text/plain;charset=US-ASCII;base64-placeholder,etc/test"
        }
      }
    ]
  }
}"#;
        fs::write(ignition_dir.join("decoded.ign"), decoded_ign).unwrap();

        let file_content = "test content";
        let file_path = ignition_dir.join("etc/test");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(file_path, file_content).unwrap();

        // Create existing target file
        fs::write(&target_file, "old ignition content").unwrap();

        // Without replace flag, should fail
        let result = assemble_ignition(&target_file, &ignition_dir, false, true, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Target file already exists"));

        // With replace flag, should succeed
        assemble_ignition(&target_file, &ignition_dir, false, true, true).unwrap();

        // Verify new content exists
        assert!(target_file.exists());
        let output_content = fs::read_to_string(&target_file).unwrap();
        assert!(output_content.contains("data:text/plain;charset=US-ASCII;base64,dGVzdCBjb250ZW50"));
        assert!(!output_content.contains("old ignition content"));
    }
}
