// SPDX-License-Identifier: Apache-2.0
use blob_test::{
    build_demo_bundle, demo_blob_path, load_manifest, sample_execution_policy, sample_memory_state,
    sample_soul_contract_snippet, unfold_blob_typed, BlobError, MockExecutionPolicy,
    MockMemoryState, SoulContractSnippet, MOCK_EXECUTION_POLICY_ID, MOCK_MEMORY_STATE_ID,
    SOUL_CONTRACT_ID,
};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_CMD_LEN: usize = 48;
const MAX_BLOB_ID_LEN: usize = 128;
const MAX_HASH_LEN: usize = 192;

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .collect()
}

fn sanitize_cli_token(raw: &str, max_len: usize, lowercase: bool) -> String {
    let mut value: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    value = value.trim().to_string();
    if lowercase {
        value = value.to_ascii_lowercase();
    }
    if value.chars().count() > max_len {
        value = value.chars().take(max_len).collect();
    }
    value
}

fn is_safe_relative_blob_path(path: &str) -> bool {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return false;
    }
    if !path.starts_with("src/") {
        return false;
    }
    !candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn is_sha256_hex(raw: &str) -> bool {
    raw.len() == 64 && raw.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  blob_test pack-demo");
    eprintln!("  blob_test manifest");
    eprintln!("  blob_test unfold <blob_id> <expected_hash>");
    eprintln!("  blob_test demo");
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{{\"ok\":false,\"error\":\"{}\"}}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), BlobError> {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args
        .first()
        .map(|value| sanitize_cli_token(value, MAX_CMD_LEN, true))
        .unwrap_or_else(|| "demo".to_string());
    match cmd {
        ref token if token == "pack-demo" => pack_demo_assets()?,
        ref token if token == "manifest" => print_manifest()?,
        ref token if token == "unfold" => {
            if args.len() < 3 {
                usage();
                return Err(BlobError::InvalidBlobId);
            }
            let blob_id = sanitize_cli_token(&args[1], MAX_BLOB_ID_LEN, false);
            let expected_hash = sanitize_cli_token(&args[2], MAX_HASH_LEN, true);
            if blob_id.is_empty() || expected_hash.is_empty() || !is_sha256_hex(&expected_hash) {
                return Err(BlobError::InvalidBlobId);
            }
            unfold_and_print(&blob_id, &expected_hash)?;
        }
        ref token if token == "demo" => run_demo()?,
        _ => {
            usage();
            return Err(BlobError::UnknownBlob(cmd));
        }
    }
    Ok(())
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn pack_demo_assets() -> Result<(), BlobError> {
    let bundle = build_demo_bundle()?;
    for blob in &bundle.blobs {
        let rel_path =
            demo_blob_path(&blob.id).ok_or_else(|| BlobError::UnknownBlob(blob.id.clone()))?;
        if !is_safe_relative_blob_path(rel_path) {
            return Err(BlobError::CompressFailed(
                "unsafe_blob_output_path".to_string(),
            ));
        }
        let abs_path = crate_root().join(rel_path);
        write_bytes(&abs_path, &blob.compressed_bytes).map_err(|e| {
            BlobError::CompressFailed(format!("write {} failed: {e}", abs_path.display()))
        })?;
        println!(
            "packed_blob id={} bytes={} sha256={}",
            blob.id,
            blob.compressed_bytes.len(),
            blob.hash
        );
    }

    let manifest_path = crate_root().join("src/manifest.blob");
    write_bytes(&manifest_path, &bundle.manifest_bytes).map_err(|e| {
        BlobError::ManifestEncodeFailed(format!("write {} failed: {e}", manifest_path.display()))
    })?;
    println!(
        "packed_manifest entries={} bytes={}",
        bundle.manifest.len(),
        bundle.manifest_bytes.len()
    );
    Ok(())
}

fn write_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

fn print_manifest() -> Result<(), BlobError> {
    let manifest = load_manifest()?;
    println!("manifest_entries={}", manifest.len());
    for entry in manifest {
        println!(
            "manifest id={} hash={} version={} signed={}",
            entry.id,
            entry.hash,
            entry.version,
            entry.signature.is_some()
        );
    }
    Ok(())
}

fn unfold_and_print(blob_id: &str, expected_hash: &str) -> Result<(), BlobError> {
    match blob_id {
        MOCK_MEMORY_STATE_ID => {
            let state: MockMemoryState = unfold_blob_typed(blob_id, expected_hash)?;
            println!(
                "unfold_ok id={} recalls={} notes={}",
                blob_id,
                state.recall_count,
                state.notes.len()
            );
        }
        MOCK_EXECUTION_POLICY_ID => {
            let policy: MockExecutionPolicy = unfold_blob_typed(blob_id, expected_hash)?;
            println!(
                "unfold_ok id={} deterministic_receipts={} max_parallel={}",
                blob_id, policy.deterministic_receipts, policy.max_parallel_workflows
            );
        }
        SOUL_CONTRACT_ID => {
            let snippet: SoulContractSnippet = unfold_blob_typed(blob_id, expected_hash)?;
            println!(
                "unfold_ok id={} covenant_version={} clauses={}",
                blob_id,
                snippet.covenant_version,
                snippet.clauses.len()
            );
        }
        _ => return Err(BlobError::UnknownBlob(blob_id.to_string())),
    }
    Ok(())
}

fn run_demo() -> Result<(), BlobError> {
    let manifest = load_manifest()?;
    println!("demo_start entries={}", manifest.len());

    for entry in &manifest {
        unfold_and_print(&entry.id, &entry.hash)?;
    }

    let expected_memory = sample_memory_state();
    let expected_policy = sample_execution_policy();
    let expected_soul = sample_soul_contract_snippet();

    let memory_entry = manifest
        .iter()
        .find(|entry| entry.id == MOCK_MEMORY_STATE_ID)
        .ok_or_else(|| BlobError::MissingManifestEntry(MOCK_MEMORY_STATE_ID.to_string()))?;
    let policy_entry = manifest
        .iter()
        .find(|entry| entry.id == MOCK_EXECUTION_POLICY_ID)
        .ok_or_else(|| BlobError::MissingManifestEntry(MOCK_EXECUTION_POLICY_ID.to_string()))?;
    let soul_entry = manifest
        .iter()
        .find(|entry| entry.id == SOUL_CONTRACT_ID)
        .ok_or_else(|| BlobError::MissingManifestEntry(SOUL_CONTRACT_ID.to_string()))?;

    let memory: MockMemoryState = unfold_blob_typed(MOCK_MEMORY_STATE_ID, &memory_entry.hash)?;
    let policy: MockExecutionPolicy =
        unfold_blob_typed(MOCK_EXECUTION_POLICY_ID, &policy_entry.hash)?;
    let soul: SoulContractSnippet = unfold_blob_typed(SOUL_CONTRACT_ID, &soul_entry.hash)?;

    println!(
        "demo_verify memory_match={} policy_match={} soul_match={}",
        memory == expected_memory,
        policy == expected_policy,
        soul == expected_soul
    );
    Ok(())
}
