# Example Usage

This directory contains example files demonstrating the use of fcos-ignition-coder.

## Example Ignition File

The `example.ign` file is a sample Fedora CoreOS Ignition configuration that contains:

1. A hostname configuration file (`/etc/hostname`)
2. A message of the day file (`/etc/motd`) with base64-encoded content
3. A setup script (`/usr/local/bin/setup.sh`) with base64-encoded content

## Try It Out

### 1. Decode the example file

```bash
cargo run -- decode -i example.ign -o ./example-decoded
```

This will create:
- `example-decoded/decoded.ign` - The modified Ignition config with file:// references
- `example-decoded/file_001` - The hostname content
- `example-decoded/file_002` - The motd content
- `example-decoded/file_003` - The setup script content

### 2. View the extracted files

```bash
cat example-decoded/file_001
cat example-decoded/file_002
cat example-decoded/file_003
```

### 3. Edit a file (optional)

```bash
echo "Modified hostname" > example-decoded/file_001
```

### 4. Encode back to Ignition format

```bash
cargo run -- encode -i example-decoded/decoded.ign -d example-decoded -o example-modified.ign
```

### 5. Compare the results

```bash
# View the re-encoded file
cat example-modified.ign

# Compare with original (should be identical if no changes were made)
diff example.ign example-modified.ign
```

## What to Expect

The decode operation extracts the embedded files and makes them easy to read and edit. The encode operation packages them back into a valid Ignition configuration that can be used to provision Fedora CoreOS systems.

The base64 content in the files decodes to:

**file_001 (hostname):**
```
my-fedora-coreos-host
```

**file_002 (motd):**
```
Welcome to Fedora CoreOS!

This system is managed by Ignition.
For more information, visit: https://coreos.github.io/ignition/
```

**file_003 (setup.sh):**
```bash
#!/bin/bash
# Initial setup script

echo "Running initial setup..."
# Add your setup commands here
echo "Setup complete!"
```
