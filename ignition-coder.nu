use std/log

# Parse a data URI and return its components
def "parse-data-url" [
    file_path: string  # The file path to save the decoded content
    uri_string: string  # The data URI string to parse
] {
    # 1. Check for the "data:" scheme prefix
    if not ($uri_string | str starts-with "data:") {
        error make --unspanned { msg: "Invalid data URI scheme." }
    }

    # 2. Split the string into the metadata and data parts at the first comma
    let parts = ($uri_string | split row "," --number 2)
    if ($parts | length) != 2 {
        error make --unspanned { msg: "Invalid data URI format - missing comma separator." }
    }

    let metadata_scheme = ($parts | first)
    let metadata_string = ($metadata_scheme | str replace "data:" "")
    let data_content = ($parts | last)

    # 3. Initialize default values
    mut mediatype = "text/plain;charset=US-ASCII"
    mut encoding = "ascii"

    # 4. Parse the metadata string
    if ($metadata_string | is-not-empty) {
        # Check for ";base64" indicator
        match ($metadata_string | split row ";" | last) {
            "base64" =>  {
                $encoding = "base64"
                let mediatype_string = ($metadata_string | str replace ";base64" "")
                if ($mediatype_string | is-not-empty) {
                    $mediatype = $mediatype_string
                }
            },
            "base64-placeholder" => {
                $encoding = "base64-placeholder"
                let mediatype_string = ($metadata_string | str replace ";base64-placeholder" "")
                if ($mediatype_string | is-not-empty) {
                        $mediatype = $mediatype_string
                }
            },
            _ => {
                $mediatype = $metadata_string
            }
        }
    }

    # 5. Convert the data based on the identified encoding
    let data = match $encoding {
        "base64" => {
            log debug $"Decoding base64: ($data_content | str substring 0..32)"
            let content = $data_content | decode base64 | decode
            log debug $"Decoded clear: ($content | str substring 0..32)"

            if ($content | str length) == 0 {
                log warning $"Empty content for file: ($file_path), creating empty file"
                "" | save -f $file_path
            } else {
                $content | save -f $file_path
            }
            ""
        },
        "base64-placeholder" => {
            let content = open --raw $file_path | to text
            log debug $"Encoding clear: ($content | str substring 0..32)"
            let encoded_content = $content | encode base64
            log debug $"Encoded base64: ($encoded_content | str substring 0..32)"
            $encoded_content
        },
        _ => {
            log warning $"Unknown encoding: ($data_content | str substring 0..32)"
            $data_content | url decode
        }
    }

    # 6. Return the parsed components
    {
        mediaType: $mediatype,
        encoding: $encoding,
        data: $data,
        prefix: $metadata_scheme,
    }
}



# Split ignition file contents into separate files
def "main decode" [
    ignition_file: path, # The ignition file to decode
    target_dir: path    # The directory to place the decoded files in
] {
    log info $"Decoding ignition file: ($ignition_file)"
    log info $"Target directory: ($target_dir)"

    if not ($ignition_file | path exists) {
        error make --unspanned { msg: $"Ignition file not found: ($ignition_file)" }
    }

    let ignition = open $ignition_file | from json
    if not ('storage' in $ignition) {
        error make --unspanned { msg: $"No decoding needed: ($ignition_file)" }
    }
    if not ('files' in $ignition.storage) {
        error make --unspanned { msg: $"No decoding needed: ($ignition_file)" }
    }

    mkdir $target_dir

    let files = $ignition.storage.files
    # $files | each { |item| $item | describe | print }
    log info $"candidate out file count: ($files | length)"
    let new_files = ($files
        # 1. Combine all filtering into one 'where' clause using 'and' and 'or'
        | where { |it|
            # Condition A: Path must not be empty
            let path_valid = ($it | get path? | default "" | is-not-empty);

            # Condition B: Check for EITHER contents.source OR append[*].source
            let source_valid = (
                # Check 1: 'contents' exists AND 'source' is in 'contents'
                ('contents' in $it and 'source' in $it.contents) or
                # Check 2: 'append' exists AND AT LEAST ONE item in 'append' has 'source'
                ('append' in $it and ($it.append | default [] | any {|item| 'source' in $item}))
            );

            $path_valid and $source_valid
        }
        # 2. Process the valid files
        | each { |file|
            let relative_path = ($file.path | str replace --regex "^/" "")

            # Determine the source field to process
            if ('contents' in $file) {
                let out_path = ($target_dir | path join $relative_path)
                log info $"out file: ($out_path)"
                mkdir ($out_path | path dirname)
                # Handle 'contents.source'
                let parsed_uri = (parse-data-url $out_path $file.contents.source)
                let new_source = $file.contents | upsert source $"data:($parsed_uri.mediaType);base64-placeholder,"
                $file | upsert contents $new_source
            } else {
                # Handle 'append' (array traversal) ---
                let updated_append = ($file.append | enumerate | each { |element|
                    # Only process items in 'append' that actually have a 'source' key
                    if ('source' in $element.item) {
                        let out_path = $target_dir | path join $relative_path ($element.index | to text)
                        mkdir ($out_path | path dirname)
                        log info $"out file: ($out_path)"
                        let parsed_uri = (parse-data-url $out_path $element.item.source)
                        $element.item | upsert source $"data:($parsed_uri.mediaType);base64-placeholder,"
                    } else {
                        $element.item
                    }
                })
                $file | upsert append $updated_append
            }
        }
    )

    let new_storage = ($ignition.storage | upsert files $new_files)
    let new_ignition = ($ignition | upsert storage $new_storage)
    let new_ignition_path = ($target_dir | path join "decoded.ign")
    log info $"writing: ($new_ignition_path)"

    $new_ignition | to json | save -f $new_ignition_path
}

# Encode ignition file contents from separate files
def "main encode" [
    target_file: path,   # The file to write the encoded ignition to
    ignition_dir: path,  # The directory containing the ignition file and file contents
] {
    log info $"Encoding ignition directory: ($ignition_dir)"
    log info $"Target ignition file: ($target_file)"
    let ign_files = (ls $ignition_dir | where name =~ '.ign$')
    if ($ign_files | length) != 1 {
        error make --unspanned { msg: $"Expected exactly one .ign file in ($ignition_dir), found ($ign_files | length)" }
    }
    let ignition_file = $ign_files | first | get name
    log info $"Source ignition file: ($ignition_file)"

    let ignition = (open $ignition_file | str trim) | from json

    let files = $ignition.storage.files
    # $files | each { |item| $item | describe | print }
    log info $"candidate in file count: ($files | length)"
    let new_files = ($files
        # 1. Combine all filtering into one 'where' clause using 'and' and 'or'
        | where { |it|
            # Condition A: Path must not be empty
            let path_valid = ($it | get path? | default "" | is-not-empty);

            # Condition B: Check for EITHER contents.source OR append[*].source
            let source_valid = (
                # Check 1: 'contents' exists AND 'source' is in 'contents'
                ('contents' in $it and 'source' in $it.contents) or
                # Check 2: 'append' exists AND AT LEAST ONE item in 'append' has 'source'
                ('append' in $it and ($it.append | default [] | any {|item| 'source' in $item}))
            );

            $path_valid and $source_valid
        }
        # 2. Process the valid files
        | each { |file|
            let relative_path = ($file.path | str replace --regex "^/" "")

            # Determine the source field to process
            if ('contents' in $file) {
                let in_path = ($ignition_dir | path join $relative_path)
                let parsed_uri = parse-data-url $in_path $file.contents.source

                if ($parsed_uri.encoding == "base64-placeholder") {
                    log info $"in file content: ($in_path)"
                    let new_source = $"data:($parsed_uri.mediaType);base64,($parsed_uri.data)"

                    let new_contents = ($file.contents | upsert source $new_source)
                    $file | upsert contents $new_contents
                }
            } else {
                # Handle 'append' (array traversal) ---
                let updated_append = ($file.append | enumerate | each { |element|
                    # Only process items in 'append' that actually have a 'source' key
                    let in_path = $ignition_dir | path join $relative_path ($element.index | to text)
                    if ('source' in $element.item) {
                        log info $"in file append: ($in_path)"
                        let parsed_uri = (parse-data-url $in_path $element.item.source)
                        $element.item | upsert source $"data:($parsed_uri.mediaType);base64,($parsed_uri.data)"
                    } else {
                        log info $"in file append no source: ($in_path)"
                        $element.item
                    }
                })
                $file | upsert append $updated_append
            }
        }
    )

    let new_storage = ($ignition.storage | upsert files $new_files)
    let new_ignition = ($ignition | upsert storage $new_storage)

    $new_ignition | to json | save -f $target_file
}

def main []  {
    log info "main encode"
}
