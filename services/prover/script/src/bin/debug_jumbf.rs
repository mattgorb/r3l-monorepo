//! Debug tool: dump the JUMBF box tree from a PNG file's caBX chunk.
//!
//! Usage: cargo run --bin debug_jumbf -- <path-to-png>

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = if args.len() > 1 {
        &args[1]
    } else {
        // Default to chatgpt.png sample
        "../../data/samples/chatgpt.png"
    };

    eprintln!("Reading: {path}");

    let file_bytes = fs::read(path).unwrap_or_else(|e| {
        eprintln!("Error reading file: {e}");
        process::exit(1);
    });

    eprintln!("File size: {} bytes", file_bytes.len());

    // Step 1: Extract caBX chunk(s) from PNG
    let jumbf = match extract_c2pa_from_png(&file_bytes) {
        Some(data) => data,
        None => {
            eprintln!("No caBX chunk found in PNG (or not a valid PNG).");
            process::exit(1);
        }
    };

    eprintln!("caBX JUMBF data: {} bytes", jumbf.len());
    eprintln!();

    // Show first 64 bytes raw
    eprintln!("=== Raw JUMBF hex (first 64 bytes) ===");
    hex_dump(&jumbf, 64);
    eprintln!();

    // Step 2: Parse and dump the full box tree
    eprintln!("=== JUMBF Box Tree ===");
    dump_boxes(&jumbf, 0);

    eprintln!();
    eprintln!("=== DEBUG: C2PA Claim CBOR Keys ===");
    debug_claim_cbor(&jumbf);
}

// ---------------------------------------------------------------------------
// PNG chunk parsing (copied from jumbf_extract.rs)
// ---------------------------------------------------------------------------

fn extract_c2pa_from_png(data: &[u8]) -> Option<Vec<u8>> {
    const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if !data.starts_with(PNG_SIG) {
        eprintln!("Not a PNG file (bad signature)");
        return None;
    }

    let mut jumbf = Vec::new();
    let mut pos = 8; // skip PNG signature
    let mut chunk_index = 0;

    while pos + 12 <= data.len() {
        let length = u32::from_be_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        let chunk_type = &data[pos + 4..pos + 8];
        let chunk_type_str = String::from_utf8_lossy(chunk_type);
        let data_start = pos + 8;
        let data_end = data_start + length;

        if data_end + 4 > data.len() {
            eprintln!(
                "  [chunk #{chunk_index}] truncated: type={chunk_type_str}, \
                 declared_len={length}, available={}",
                data.len() - data_start
            );
            break;
        }

        eprintln!(
            "  PNG chunk #{chunk_index}: type={chunk_type_str}, len={length}, offset={pos}"
        );

        if chunk_type == b"caBX" {
            eprintln!("    -> caBX found! Appending {length} bytes of JUMBF data");
            jumbf.extend_from_slice(&data[data_start..data_end]);
        }

        pos = data_end + 4; // skip CRC
        chunk_index += 1;
    }

    if jumbf.is_empty() {
        None
    } else {
        Some(jumbf)
    }
}

// ---------------------------------------------------------------------------
// JUMBF / ISO BMFF box parsing & dumping
// ---------------------------------------------------------------------------

fn dump_boxes(data: &[u8], depth: usize) {
    let indent = "  ".repeat(depth);
    let mut pos = 0;
    let mut index = 0;

    while pos + 8 <= data.len() {
        let raw_size = u32::from_be_bytes(
            data[pos..pos + 4].try_into().unwrap_or([0; 4]),
        ) as usize;
        let box_type_bytes: [u8; 4] =
            data[pos + 4..pos + 8].try_into().unwrap_or([0; 4]);
        let box_type_str = String::from_utf8_lossy(&box_type_bytes);

        // Handle extended size (size == 1 means 64-bit size follows)
        let (header_size, box_size) = if raw_size == 1 && pos + 16 <= data.len() {
            let ext_size = u64::from_be_bytes(
                data[pos + 8..pos + 16].try_into().unwrap_or([0; 8]),
            ) as usize;
            (16, ext_size)
        } else if raw_size == 0 {
            // size 0 means box extends to end of data
            (8, data.len() - pos)
        } else {
            (8, raw_size)
        };

        if box_size < header_size {
            println!(
                "{indent}[#{index}] INVALID: type={box_type_str}, raw_size={raw_size}, \
                 header_size={header_size} (box_size < header)"
            );
            break;
        }

        if pos + box_size > data.len() {
            println!(
                "{indent}[#{index}] TRUNCATED: type={box_type_str}, declared_size={box_size}, \
                 available={}, offset={pos}",
                data.len() - pos
            );
            // Still try to show what we have
            let available = &data[pos + header_size..data.len().min(pos + box_size)];
            show_box_content(&indent, &box_type_str, available, depth);
            break;
        }

        let content = &data[pos + header_size..pos + box_size];

        println!(
            "{indent}[#{index}] type=\"{box_type_str}\"  size={box_size}  \
             content_len={}  offset={pos}",
            content.len()
        );

        show_box_content(&indent, &box_type_str, content, depth);

        pos += box_size;
        index += 1;
    }

    if index == 0 && !data.is_empty() {
        println!("{indent}(no valid boxes found; raw data {} bytes)", data.len());
        println!("{indent}  hex: {}", hex_string(&data[..data.len().min(40)]));
    }
}

fn show_box_content(indent: &str, box_type: &str, content: &[u8], depth: usize) {
    // Show first 20 bytes hex for every box
    let preview_len = content.len().min(20);
    println!(
        "{indent}  hex[0..{preview_len}]: {}",
        hex_string(&content[..preview_len])
    );

    match box_type {
        "jumd" => {
            // Parse JUMD description box
            parse_and_show_jumd(indent, content);
        }
        "jumb" => {
            // Superbox: recurse into children
            println!("{indent}  (superbox — recursing into children)");
            dump_boxes(content, depth + 1);
        }
        "uuid" => {
            println!("{indent}  (UUID box)");
            if content.len() >= 16 {
                println!(
                    "{indent}  uuid: {}",
                    format_uuid(&content[..16])
                );
            }
        }
        "bfdb" => {
            println!("{indent}  (embedded file data box)");
            if !content.is_empty() {
                let toggle = content[0];
                println!("{indent}  toggle_byte: 0x{toggle:02x}");
                let mut cpos = 1;
                if toggle & 0x01 != 0 {
                    let (s, next) = read_null_terminated(&content[cpos..]);
                    println!("{indent}  media_type: \"{s}\"");
                    cpos += next;
                }
                if toggle & 0x02 != 0 {
                    let (s, next) = read_null_terminated(&content[cpos..]);
                    println!("{indent}  file_name: \"{s}\"");
                    cpos += next;
                }
                println!(
                    "{indent}  payload_offset: {cpos}, payload_len: {}",
                    content.len() - cpos
                );
            }
        }
        "cbor" => {
            println!("{indent}  (CBOR data box, {} bytes)", content.len());
        }
        "json" => {
            let snippet = String::from_utf8_lossy(
                &content[..content.len().min(200)],
            );
            println!("{indent}  json_preview: {snippet}");
        }
        _ => {
            // For unknown boxes, try to detect if they contain sub-boxes
            if content.len() >= 8 {
                let maybe_size = u32::from_be_bytes(
                    content[0..4].try_into().unwrap_or([0; 4]),
                ) as usize;
                let maybe_type = &content[4..8];
                let looks_like_boxes = maybe_size >= 8
                    && maybe_size <= content.len()
                    && maybe_type.iter().all(|&b| b.is_ascii_graphic());
                if looks_like_boxes {
                    println!("{indent}  (might contain sub-boxes; first child looks like size={maybe_size} type=\"{}\")",
                        String::from_utf8_lossy(maybe_type));
                }
            }
        }
    }
}

fn parse_and_show_jumd(indent: &str, data: &[u8]) {
    if data.len() < 16 {
        println!("{indent}  jumd: too short ({} bytes)", data.len());
        return;
    }

    let uuid = &data[0..16];
    println!("{indent}  jumd_uuid: {}", format_uuid(uuid));

    // Known C2PA UUIDs
    let uuid_hex = hex_string(uuid);
    let _known = match uuid_hex.as_str() {
        "6332706100110010800000aa00389b71" => "c2pa (manifest store)",
        "6332706d00110010800000aa00389b71" => "c2pa manifest",
        "6332636c00110010800000aa00389b71" => "c2pa.claim",
        "6332637300110010800000aa00389b71" => "c2pa.signature (COSE_Sign1)",
        "63326173 00110010800000aa00389b71" => "c2pa.assertions",
        _ => "(unknown)",
    };
    // do a cleaner check by removing spaces
    let uuid_hex_clean: String = uuid_hex.chars().filter(|c| !c.is_whitespace()).collect();
    let known_clean = match uuid_hex_clean.as_str() {
        "6332706100110010800000aa00389b71" => "c2pa (manifest store)",
        "6332706d00110010800000aa00389b71" => "c2pa manifest",
        "6332636c00110010800000aa00389b71" => "c2pa.claim",
        "6332637300110010800000aa00389b71" => "c2pa.signature (COSE_Sign1)",
        "6332617300110010800000aa00389b71" => "c2pa.assertions",
        "6332687400110010800000aa00389b71" => "c2pa.hash.data",
        _ => "(unknown)",
    };
    println!("{indent}  jumd_type: {known_clean}");

    if data.len() > 16 {
        let toggles = data[16];
        println!("{indent}  toggles: 0x{toggles:02x}");

        let has_label = toggles & 0x02 != 0;
        let has_id = toggles & 0x04 != 0;
        let has_hash = toggles & 0x08 != 0;

        println!(
            "{indent}  flags: requestable={}, label={has_label}, id={has_id}, hash={has_hash}",
            toggles & 0x01 != 0
        );

        if has_label && data.len() > 17 {
            let (label, _) = read_null_terminated(&data[17..]);
            println!("{indent}  label: \"{label}\"");
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hex_string(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn hex_dump(data: &[u8], max_bytes: usize) {
    let end = data.len().min(max_bytes);
    for row_start in (0..end).step_by(16) {
        let row_end = (row_start + 16).min(end);
        let hex_part: String = data[row_start..row_end]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii_part: String = data[row_start..row_end]
            .iter()
            .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
            .collect();
        eprintln!("  {row_start:06x}: {hex_part:<48} {ascii_part}");
    }
}

fn format_uuid(data: &[u8]) -> String {
    if data.len() < 16 {
        return hex_string(data);
    }
    format!(
        "{}-{}-{}-{}-{}",
        hex_string(&data[0..4]).replace(' ', ""),
        hex_string(&data[4..6]).replace(' ', ""),
        hex_string(&data[6..8]).replace(' ', ""),
        hex_string(&data[8..10]).replace(' ', ""),
        hex_string(&data[10..16]).replace(' ', ""),
    )
}

fn read_null_terminated(data: &[u8]) -> (String, usize) {
    match data.iter().position(|&b| b == 0) {
        Some(pos) => {
            let s = String::from_utf8_lossy(&data[..pos]).to_string();
            (s, pos + 1)
        }
        None => {
            let s = String::from_utf8_lossy(data).to_string();
            (s, data.len())
        }
    }
}

// ---------------------------------------------------------------------------
// DEBUG: Extract and parse c2pa.claim.v2 CBOR to show keys
// ---------------------------------------------------------------------------

fn debug_claim_cbor(jumbf_data: &[u8]) {
    // The JUMBF data is structured as:
    // - Top-level jumb (c2pa manifest store)
    //   - Child jumb boxes (each is a manifest)

    // First, we need to skip into the top-level c2pa store box
    let store_content = skip_to_store_content(jumbf_data);
    let store_content = match store_content {
        Some(data) => data,
        None => {
            eprintln!("Could not find c2pa store content");
            return;
        }
    };

    // Now find all manifests within the store
    let manifests = find_manifests(&store_content);
    if manifests.is_empty() {
        eprintln!("No manifests found in c2pa store");
        return;
    }

    eprintln!("Found {} manifest(s), using last (active) one", manifests.len());
    let active_manifest_data = manifests.last().unwrap();

    // 2. Find the c2pa.claim box in the manifest
    let claim_cbor = find_claim_cbor(active_manifest_data);
    let claim_cbor = match claim_cbor {
        Some(data) => data,
        None => {
            eprintln!("No c2pa.claim cbor box found in active manifest");
            return;
        }
    };

    eprintln!("Found c2pa.claim cbor box: {} bytes", claim_cbor.len());
    eprintln!();

    // 3. Parse the CBOR and dump keys
    parse_and_dump_cbor_keys(&claim_cbor);
}

/// Skip to the content of the top-level c2pa store box
fn skip_to_store_content(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 8 {
        return None;
    }

    let raw_size = u32::from_be_bytes(
        data[0..4].try_into().ok()?,
    ) as usize;
    let box_type = &data[4..8];

    if box_type != b"jumb" {
        return None;
    }

    let (header_size, box_size) = if raw_size == 1 && data.len() >= 16 {
        let ext_size = u64::from_be_bytes(
            data[8..16].try_into().ok()?,
        ) as usize;
        (16, ext_size)
    } else if raw_size == 0 {
        (8, data.len())
    } else {
        (8, raw_size)
    };

    if box_size > data.len() {
        return None;
    }

    let content = &data[header_size..box_size];
    Some(content.to_vec())
}

/// Find all manifests in the JUMBF data (each manifest is a jumb superbox)
fn find_manifests(data: &[u8]) -> Vec<Vec<u8>> {
    let mut manifests = Vec::new();
    let mut pos = 0;

    while pos + 8 <= data.len() {
        let raw_size = u32::from_be_bytes(
            data[pos..pos + 4].try_into().unwrap_or([0; 4]),
        ) as usize;
        let box_type = &data[pos + 4..pos + 8];

        let (header_size, box_size) = if raw_size == 1 && pos + 16 <= data.len() {
            let ext_size = u64::from_be_bytes(
                data[pos + 8..pos + 16].try_into().unwrap_or([0; 8]),
            ) as usize;
            (16, ext_size)
        } else if raw_size == 0 {
            (8, data.len() - pos)
        } else {
            (8, raw_size)
        };

        if pos + box_size > data.len() {
            break;
        }

        // Look for jumb boxes (manifests are jumb boxes containing a c2pa.claim)
        if box_type == b"jumb" {
            let content = &data[pos + header_size..pos + box_size];
            manifests.push(content.to_vec());
        }

        pos += box_size;
    }

    manifests
}

/// Find the c2pa.claim cbor box in a manifest
fn find_claim_cbor(manifest_data: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 0;

    while pos + 8 <= manifest_data.len() {
        let raw_size = u32::from_be_bytes(
            manifest_data[pos..pos + 4].try_into().unwrap_or([0; 4]),
        ) as usize;
        let box_type = &manifest_data[pos + 4..pos + 8];

        let (header_size, box_size) = if raw_size == 1 && pos + 16 <= manifest_data.len() {
            let ext_size = u64::from_be_bytes(
                manifest_data[pos + 8..pos + 16].try_into().unwrap_or([0; 8]),
            ) as usize;
            (16, ext_size)
        } else if raw_size == 0 {
            (8, manifest_data.len() - pos)
        } else {
            (8, raw_size)
        };

        if pos + box_size > manifest_data.len() {
            break;
        }

        let content = &manifest_data[pos + header_size..pos + box_size];

        // Look for jumb boxes (nested claim or assertion)
        if box_type == b"jumb" {
            // Check if this is a c2pa.claim box
            if is_claim_box(content) {
                // Now find the cbor box inside
                return find_cbor_box(content);
            }
        }

        pos += box_size;
    }

    None
}

/// Check if a jumb box is a c2pa.claim (by checking jumd UUID)
fn is_claim_box(jumb_content: &[u8]) -> bool {
    if jumb_content.len() < 8 {
        return false;
    }

    let first_box_size = u32::from_be_bytes(
        jumb_content[0..4].try_into().unwrap_or([0; 4]),
    ) as usize;
    let first_box_type = &jumb_content[4..8];

    if first_box_type != b"jumd" {
        return false;
    }

    if first_box_size < 24 {
        return false;
    }

    let uuid = &jumb_content[8..24];
    let uuid_hex = hex_string(uuid).chars().filter(|c| !c.is_whitespace()).collect::<String>();

    // c2pa.claim UUID: 6332636c00110010800000aa00389b71
    uuid_hex == "6332636c00110010800000aa00389b71"
}

/// Find the cbor box inside a claim jumb
fn find_cbor_box(jumb_content: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 0;

    while pos + 8 <= jumb_content.len() {
        let raw_size = u32::from_be_bytes(
            jumb_content[pos..pos + 4].try_into().unwrap_or([0; 4]),
        ) as usize;
        let box_type = &jumb_content[pos + 4..pos + 8];

        let (header_size, box_size) = if raw_size == 1 && pos + 16 <= jumb_content.len() {
            let ext_size = u64::from_be_bytes(
                jumb_content[pos + 8..pos + 16].try_into().unwrap_or([0; 8]),
            ) as usize;
            (16, ext_size)
        } else if raw_size == 0 {
            (8, jumb_content.len() - pos)
        } else {
            (8, raw_size)
        };

        if pos + box_size > jumb_content.len() {
            break;
        }

        if box_type == b"cbor" {
            let content = &jumb_content[pos + header_size..pos + box_size];
            return Some(content.to_vec());
        }

        pos += box_size;
    }

    None
}

/// Parse CBOR data and dump all keys and values
fn parse_and_dump_cbor_keys(cbor_data: &[u8]) {
    use ciborium::Value;

    let value: Value = match ciborium::from_reader(cbor_data) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse CBOR: {e}");
            eprintln!("Raw CBOR hex (first 100 bytes):");
            hex_dump(cbor_data, 100);
            return;
        }
    };

    eprintln!("Successfully parsed CBOR!");
    eprintln!();
    dump_cbor_value(&value, 0);
}

/// Recursively dump CBOR value structure
fn dump_cbor_value(value: &ciborium::Value, depth: usize) {
    use ciborium::Value;
    let indent = "  ".repeat(depth);

    match value {
        Value::Map(map) => {
            eprintln!("{indent}Map with {} entries:", map.len());
            for (key, val) in map {
                eprint!("{indent}  Key: ");
                match key {
                    Value::Text(s) => eprint!("\"{s}\""),
                    Value::Integer(i) => eprint!("{i:?}"),
                    Value::Bytes(b) => eprint!("Bytes({})", hex_string(b)),
                    _ => eprint!("{key:?}"),
                }
                eprintln!();

                eprint!("{indent}  Value: ");
                match val {
                    Value::Text(s) => eprintln!("\"{s}\""),
                    Value::Integer(i) => eprintln!("{i:?}"),
                    Value::Bool(b) => eprintln!("{b}"),
                    Value::Null => eprintln!("null"),
                    Value::Bytes(b) => eprintln!("Bytes({} bytes): {}", b.len(), hex_string(&b[..b.len().min(40)])),
                    Value::Array(arr) => {
                        eprintln!("Array[{}]:", arr.len());
                        for (i, item) in arr.iter().enumerate() {
                            eprintln!("{indent}    [{i}]:");
                            dump_cbor_value(item, depth + 3);
                        }
                    }
                    Value::Map(_) => {
                        eprintln!();
                        dump_cbor_value(val, depth + 2);
                    }
                    _ => eprintln!("{val:?}"),
                }
                eprintln!();
            }
        }
        Value::Array(arr) => {
            eprintln!("{indent}Array with {} items:", arr.len());
            for (i, item) in arr.iter().enumerate() {
                eprintln!("{indent}  [{i}]:");
                dump_cbor_value(item, depth + 2);
            }
        }
        Value::Text(s) => eprintln!("{indent}\"{s}\""),
        Value::Integer(i) => eprintln!("{indent}{i:?}"),
        Value::Bool(b) => eprintln!("{indent}{b}"),
        Value::Null => eprintln!("{indent}null"),
        Value::Bytes(b) => eprintln!("{indent}Bytes({} bytes)", b.len()),
        _ => eprintln!("{indent}{value:?}"),
    }
}
