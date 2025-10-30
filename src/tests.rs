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

        disassemble_ignition(&input_path, &decoded_dir).unwrap();

        assert!(decoded_dir.join("decoded.ign").exists());
        assert!(decoded_dir.join("etc/test").exists());

        let decoded_content = fs::read_to_string(decoded_dir.join("etc/test")).unwrap();
        assert_eq!(decoded_content, "test content");

        let decoded_ign_content = fs::read_to_string(decoded_dir.join("decoded.ign")).unwrap();
        assert!(
            decoded_ign_content.contains("data:text/plain;charset=US-ASCII;base64-placeholder,")
        );
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
          "source": "data:text/plain;charset=US-ASCII;base64-placeholder,"
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

        assemble_ignition(&target_file, &ignition_dir, false, true).unwrap();

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

        disassemble_ignition(&input_path, &decoded_dir).unwrap();
        assemble_ignition(&output_path, &decoded_dir, false, true).unwrap();

        let input_json: serde_json::Value = serde_json::from_str(test_ignition).unwrap();
        let output_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(output_path).unwrap()).unwrap();

        assert_eq!(input_json, output_json);
    }
}
