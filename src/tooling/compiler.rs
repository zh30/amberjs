//! Single Executable Application (SEA) compiler for Beejs.
//!
//! Merges JavaScript/TypeScript code with the Beejs runtime binary into a single,
//! zero-dependency executable without needing an external compiler or toolchain.

use anyhow::{anyhow, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

/// 16-byte magic identifier trailer appended at the very end of compiled binaries.
pub const MAGIC_TRAILER: &[u8; 16] = b"BEE_STANDALONE\0\0";

/// Trailer size: payload_len (8 bytes) + flags (8 bytes) + MAGIC_TRAILER (16 bytes) = 32 bytes
pub const TRAILER_TOTAL_SIZE: u64 = 32;

/// Checks whether the current executable contains an embedded standalone script.
pub fn detect_standalone_payload() -> Result<Option<String>> {
    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow!("Failed to resolve current executable: {}", e))?;

    let mut file = match File::open(&current_exe) {
        Ok(f) => f,
        Err(_) => return Ok(None),
    };

    let total_len = file.metadata()?.len();
    if total_len < TRAILER_TOTAL_SIZE {
        return Ok(None);
    }

    // Check magic trailer
    file.seek(SeekFrom::End(-16))?;
    let mut magic_buf = [0u8; 16];
    file.read_exact(&mut magic_buf)?;

    if &magic_buf != MAGIC_TRAILER {
        return Ok(None);
    }

    // Read payload length
    file.seek(SeekFrom::End(-(TRAILER_TOTAL_SIZE as i64)))?;
    let mut len_buf = [0u8; 8];
    file.read_exact(&mut len_buf)?;
    let payload_len = u64::from_le_bytes(len_buf);

    if payload_len == 0 || payload_len > total_len - TRAILER_TOTAL_SIZE {
        return Ok(None);
    }

    // Read the script payload
    let offset_from_end = (TRAILER_TOTAL_SIZE + payload_len) as i64;
    file.seek(SeekFrom::End(-offset_from_end))?;
    let mut script_buf = vec![0u8; payload_len as usize];
    file.read_exact(&mut script_buf)?;

    let script = String::from_utf8(script_buf)
        .map_err(|e| anyhow!("Corrupted standalone payload (non UTF-8): {}", e))?;

    Ok(Some(script))
}

/// Resolves the base Beejs runtime binary to clone.
pub fn resolve_runtime_binary() -> Result<std::path::PathBuf> {
    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow!("Failed to resolve current executable: {}", e))?;

    let file_name = current_exe
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if file_name == "bee" || file_name == "bee.exe" {
        return Ok(current_exe);
    }

    // In tests (where current_exe is in deps/), find the sibling `bee` binary
    if let Some(parent) = current_exe.parent() {
        let candidate = parent.join("bee");
        if candidate.is_file() {
            return Ok(candidate);
        }
        if let Some(grandparent) = parent.parent() {
            let candidate2 = grandparent.join("bee");
            if candidate2.is_file() {
                return Ok(candidate2);
            }
        }
    }

    Ok(current_exe)
}

/// Compiles an entry JS/TS script into a standalone self-executing binary.
pub fn compile_binary(entry_file: &Path, output_path: &Path) -> Result<()> {
    if !entry_file.exists() {
        return Err(anyhow!("Entry file '{}' not found", entry_file.display()));
    }

    // Bundle the entry file and all its dependencies into a self-contained JS bundle
    let bundle_opts = crate::tooling::bundler::BundleOptions {
        entry: entry_file.to_path_buf(),
        outfile: None,
        minify: false,
        sourcemap: false,
        target: "es2022".to_string(),
        import_map: None,
    };
    let bundle_out = crate::tooling::bundler::bundle_project(&bundle_opts)?;
    let runnable_code = bundle_out.code;

    let runtime_binary = resolve_runtime_binary()?;

    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Copy runtime executable to destination
    fs::copy(&runtime_binary, output_path).map_err(|e| {
        anyhow!(
            "Failed to create binary at '{}': {}",
            output_path.display(),
            e
        )
    })?;

    // Append payload + metadata + trailer
    let payload_bytes = runnable_code.as_bytes();
    let payload_len = payload_bytes.len() as u64;
    let flags: u64 = 0;

    let mut out_file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(output_path)?;

    out_file.write_all(payload_bytes)?;
    out_file.write_all(&payload_len.to_le_bytes())?;
    out_file.write_all(&flags.to_le_bytes())?;
    out_file.write_all(MAGIC_TRAILER)?;
    out_file.flush()?;

    // Mark as executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(output_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(output_path, perms)?;
    }

    // Re-sign with ad-hoc signature on macOS so Gatekeeper allows execution
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("codesign")
            .args(["--sign", "-", "--force", output_path.to_str().unwrap_or("")])
            .output();
    }

    println!(
        "✅ Standalone binary successfully compiled: {}",
        output_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_standalone_embed_and_extract() {
        let dir = tempdir().expect("tempdir");
        let fake_exe = dir.path().join("fake_bin");
        fs::write(&fake_exe, b"BASE_BINARY_BYTES").expect("write");

        let script = "console.log('Hello Standalone');";
        let payload_bytes = script.as_bytes();
        let payload_len = payload_bytes.len() as u64;
        let flags: u64 = 0;

        let mut f = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&fake_exe)
            .expect("open");
        f.write_all(payload_bytes).expect("write");
        f.write_all(&payload_len.to_le_bytes()).expect("write");
        f.write_all(&flags.to_le_bytes()).expect("write");
        f.write_all(MAGIC_TRAILER).expect("write");
        f.flush().expect("flush");

        // Verify extraction logic on custom file
        let mut file = File::open(&fake_exe).expect("open");
        file.seek(SeekFrom::End(-16)).expect("seek");
        let mut magic_buf = [0u8; 16];
        file.read_exact(&mut magic_buf).expect("read");
        assert_eq!(&magic_buf, MAGIC_TRAILER);

        file.seek(SeekFrom::End(-(TRAILER_TOTAL_SIZE as i64)))
            .expect("seek");
        let mut len_buf = [0u8; 8];
        file.read_exact(&mut len_buf).expect("read");
        let extracted_len = u64::from_le_bytes(len_buf);
        assert_eq!(extracted_len, payload_len);

        let offset = (TRAILER_TOTAL_SIZE + extracted_len) as i64;
        file.seek(SeekFrom::End(-offset)).expect("seek");
        let mut read_script = vec![0u8; extracted_len as usize];
        file.read_exact(&mut read_script).expect("read");
        assert_eq!(String::from_utf8(read_script).unwrap(), script);
    }
}
