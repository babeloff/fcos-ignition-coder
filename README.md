# fcos-ignition-coder

This program will 'expand' (decode) an FCOS ignition file and 'assemble' (encode) the expanded form.

## Overview

`fcos-ignition-coder` is a command-line utility for working with Fedora CoreOS (FCOS) Ignition configuration files. It provides two main operations:

- **Decode**: Extract embedded files from an Ignition configuration file
- **Encode**: Package extracted files back into an Ignition configuration file

This is useful for:
- Editing files within Ignition configs more easily
- Version controlling the actual file contents instead of base64-encoded data
- Understanding and debugging Ignition configurations

## Installation

### Using pixi (Recommended)

This project uses [pixi](https://pixi.sh/) for dependency management. If you have pixi installed:

```bash
pixi install
pixi run build
```

### Using Cargo

Alternatively, you can build with cargo directly:

```bash
cargo build --release
```

The binary will be available at `target/release/fcos-ignition-coder`.

## Usage

### Decode Command

Extract embedded files from an Ignition configuration:

```bash
fcos-ignition-coder decode --input <INPUT_FILE> --output <OUTPUT_DIR>
```

**Example:**
```bash
fcos-ignition-coder decode --input config.ign --output ./decoded
```

This will:
1. Read the Ignition file from `config.ign`
2. Extract all embedded files (typically base64-encoded data URLs)
3. Save each extracted file as `file_001`, `file_002`, etc. in the `./decoded` directory
4. Generate a `decoded.ign` file where the original file contents are replaced with `file://./file_XXX` placeholders

### Encode Command

Re-encode extracted files back into an Ignition configuration:

```bash
fcos-ignition-coder encode --input <DECODED_IGN> --files-dir <FILES_DIR> --output <OUTPUT_FILE>
```

**Example:**
```bash
fcos-ignition-coder encode --input ./decoded/decoded.ign --files-dir ./decoded --output config-new.ign
```

This will:
1. Read the `decoded.ign` file
2. For each `file://./file_XXX` reference, read the corresponding file from the files directory
3. Encode the file contents as base64 data URLs
4. Generate a complete Ignition configuration file at `config-new.ign`

## Example Workflow

1. **Decode an existing Ignition file:**
   ```bash
   fcos-ignition-coder decode -i myconfig.ign -o ./work
   ```

2. **Edit the extracted files:**
   ```bash
   # Edit files in ./work/file_001, ./work/file_002, etc.
   vim ./work/file_001
   ```

3. **Optionally edit the decoded.ign structure:**
   ```bash
   vim ./work/decoded.ign
   ```

4. **Encode back to Ignition format:**
   ```bash
   fcos-ignition-coder encode -i ./work/decoded.ign -d ./work -o myconfig-modified.ign
   ```

## Supported Ignition Versions

This tool supports Ignition config versions:
- 3.0.0
- 3.1.0
- 3.2.0
- 3.3.0
- 3.4.0
- 3.5.0

## Development

### Running Tests

```bash
cargo test
```

### Building with pixi

```bash
pixi run build    # Build the project
pixi run test     # Run tests
```

## License

Licensed under the Apache License, Version 2.0. See LICENSE file for details.
