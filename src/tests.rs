#[cfg(test)]
mod tests {
    use crate::{decode_data_url, decode_ignition, encode_ignition};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_decode_encode_roundtrip() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("test.ign");
        let decoded_dir = temp_dir.path().join("decoded");
        let output_path = temp_dir.path().join("output.ign");

        // Create a test Ignition file
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

        // Test decode
        decode_ignition(&input_path, &decoded_dir).unwrap();

        // Verify decoded files exist
        assert!(decoded_dir.join("decoded.ign").exists());
        assert!(decoded_dir.join("file_001").exists());

        // Verify decoded file content
        let decoded_content = fs::read_to_string(decoded_dir.join("file_001")).unwrap();
        assert_eq!(decoded_content, "test content");

        // Test encode
        let decoded_ign_path = decoded_dir.join("decoded.ign");
        encode_ignition(&decoded_ign_path, &decoded_dir, &output_path).unwrap();

        // Verify output file exists
        assert!(output_path.exists());

        // Verify the output contains base64 encoded data
        let output_content = fs::read_to_string(&output_path).unwrap();
        assert!(output_content.contains("data:;base64,"));
        assert!(output_content.contains("dGVzdCBjb250ZW50"));
    }

    #[test]
    fn test_decode_data_url_base64() {
        let data_url = "data:;base64,SGVsbG8gV29ybGQh";
        let result = decode_data_url(data_url).unwrap();
        assert_eq!(result, b"Hello World!");
    }

    #[test]
    fn test_decode_data_url_with_mime_type() {
        let data_url = "data:text/plain;base64,SGVsbG8gV29ybGQh";
        let result = decode_data_url(data_url).unwrap();
        assert_eq!(result, b"Hello World!");
    }

    #[test]
    fn test_decode_data_url_plain_text() {
        let data_url = "data:,Hello%20World";
        let result = decode_data_url(data_url).unwrap();
        // URL encoding is not decoded, just returned as-is
        assert_eq!(result, b"Hello%20World");
    }
}
