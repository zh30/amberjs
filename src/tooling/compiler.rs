//! Single Executable Application (SEA) compiler for `amber compile`.
//!
//! The output is a copy of the host `amber` executable with a bundled script
//! and a 32-byte trailer appended. It is not a freestanding or cross-compiled
//! binary. The user-facing contract is `docs/COMPILE_CONTRACT.md`.

use anyhow::{anyhow, Result};
use oxc::allocator::Allocator;
use oxc::ast::ast::{Argument, CallExpression, Expression};
use oxc::ast_visit::Visit;
use oxc::parser::Parser;
use oxc::span::SourceType;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// 16-byte magic identifier at the very end of a compiled SEA binary.
/// This is not an environment variable. Setting `AMBER_STANDALONE` in the
/// environment does not select standalone mode.
pub const MAGIC_TRAILER: &[u8; 16] = b"AMBER_STANDALONE";

/// Trailer size: payload_len (u64 LE) + flags (u64 LE) + magic (16) = 32 bytes.
pub const TRAILER_TOTAL_SIZE: u64 = 32;

/// Stable prefix for `amber compile` failures. The CLI prints this verbatim.
pub const COMPILE_ERROR_PREFIX: &str = "error: amber compile:";

/// Stable prefix for a SEA binary whose trailer is present but unusable,
/// and for a payload that fails while it is running.
pub const STANDALONE_ERROR_PREFIX: &str = "error: amber standalone:";

const EMBEDDABLE_EXTENSIONS: &[&str] = &["js", "mjs", "cjs", "ts", "tsx", "mts", "json"];
const EMBEDDABLE_LIST: &str = "js, mjs, cjs, ts, tsx, mts, or json";

fn compile_error(message: impl std::fmt::Display) -> anyhow::Error {
    anyhow!("{COMPILE_ERROR_PREFIX} {message}")
}

fn standalone_error(message: impl std::fmt::Display) -> anyhow::Error {
    anyhow!("{STANDALONE_ERROR_PREFIX} {message}")
}

fn is_amber_executable_name(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("amber" | "amber.exe")
    )
}

fn runtime_candidate_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["amber.exe", "amber"]
    } else {
        &["amber"]
    }
}

/// Location of a SEA trailer inside a file image.
///
/// `trailer_end` is the exclusive end of the 16-byte magic. `payload_len` is
/// stored at `trailer_end - 32` and `flags` at `trailer_end - 24`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandaloneTrailer {
    pub payload_start: usize,
    pub payload_len: u64,
    pub flags: u64,
    pub trailer_end: usize,
}

/// Reads a SEA payload from `path`.
///
/// `Ok(None)` means the file is not a SEA binary. `Err` means a trailer is
/// present and is not a payload this runtime can run.
pub fn read_standalone_payload(path: &Path) -> Result<Option<String>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    let Some(trailer) = find_standalone_trailer(&bytes)? else {
        return Ok(None);
    };
    if trailer.flags != 0 {
        return Err(standalone_error(format!(
            "unsupported trailer flags: {}",
            trailer.flags
        )));
    }
    let payload =
        &bytes[trailer.payload_start..trailer.payload_start + trailer.payload_len as usize];
    let script =
        std::str::from_utf8(payload).map_err(|_| standalone_error("payload is not UTF-8"))?;
    Ok(Some(script.to_string()))
}

/// Finds a SEA trailer at EOF (Linux, Windows, and raw fixtures) or in a
/// Mach-O `__AMBER` segment (macOS, where an ad-hoc code signature follows
/// `__LINKEDIT` and the magic is not the last bytes of the file).
pub fn find_standalone_trailer(bytes: &[u8]) -> Result<Option<StandaloneTrailer>> {
    if let Some(segment) = macho_amber_segment(bytes) {
        let end = segment.end;
        if end < TRAILER_TOTAL_SIZE as usize || end > bytes.len() {
            return Err(standalone_error("invalid trailer"));
        }
        if &bytes[end - 16..end] != MAGIC_TRAILER {
            return Err(standalone_error("invalid trailer"));
        }
        return parse_trailer_at(bytes, end, Some(segment.start));
    }
    if bytes.len() >= 16 && bytes.ends_with(MAGIC_TRAILER) {
        return parse_trailer_at(bytes, bytes.len(), None);
    }
    Ok(None)
}

fn parse_trailer_at(
    bytes: &[u8],
    trailer_end: usize,
    segment_start: Option<usize>,
) -> Result<Option<StandaloneTrailer>> {
    if trailer_end < TRAILER_TOTAL_SIZE as usize || trailer_end > bytes.len() {
        return Err(standalone_error("invalid trailer"));
    }
    let len_at = trailer_end - 32;
    let payload_len = u64::from_le_bytes(bytes[len_at..len_at + 8].try_into().unwrap());
    let flags = u64::from_le_bytes(bytes[len_at + 8..len_at + 16].try_into().unwrap());
    if flags != 0 {
        return Ok(Some(StandaloneTrailer {
            payload_start: 0,
            payload_len,
            flags,
            trailer_end,
        }));
    }
    if payload_len == 0 {
        return Err(standalone_error("invalid payload length"));
    }
    let payload_len_us =
        usize::try_from(payload_len).map_err(|_| standalone_error("invalid payload length"))?;
    if payload_len_us > len_at {
        return Err(standalone_error("invalid payload length"));
    }
    let payload_start = len_at - payload_len_us;
    if let Some(start) = segment_start {
        if payload_start < start {
            return Err(standalone_error("invalid payload length"));
        }
    }
    Ok(Some(StandaloneTrailer {
        payload_start,
        payload_len,
        flags,
        trailer_end,
    }))
}

/// Checks whether the current executable contains an embedded standalone script.
pub fn detect_standalone_payload() -> Result<Option<String>> {
    let Ok(current_exe) = std::env::current_exe() else {
        return Ok(None);
    };
    read_standalone_payload(&current_exe)
}

/// Resolves the host Amber runtime binary to clone.
///
/// The CLI uses the running `amber` / `amber.exe`. Tests and other library
/// callers look for a sibling binary with that name. A renamed executable
/// falls back to itself so `amber compile` still embeds the runtime that is
/// actually running.
pub fn resolve_runtime_binary() -> Result<PathBuf> {
    let current_exe = std::env::current_exe()
        .map_err(|err| compile_error(format!("failed to resolve current executable: {err}")))?;

    if is_amber_executable_name(&current_exe) {
        return Ok(current_exe);
    }

    let mut search_dirs = Vec::new();
    if let Some(parent) = current_exe.parent() {
        search_dirs.push(parent.to_path_buf());
        if let Some(grandparent) = parent.parent() {
            search_dirs.push(grandparent.to_path_buf());
        }
    }
    for dir in search_dirs {
        for name in runtime_candidate_names() {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Ok(current_exe)
}

fn ensure_supported_host() -> Result<()> {
    match std::env::consts::OS {
        "linux" | "macos" | "windows" => Ok(()),
        other => Err(compile_error(format!(
            "unsupported host OS '{other}' (supported hosts: linux, macos, windows; cross-compilation is not supported)"
        ))),
    }
}

fn unsupported_module(path: &Path) -> anyhow::Error {
    compile_error(format!(
        "unsupported module '{}' (embedded files must be {EMBEDDABLE_LIST})",
        path.display()
    ))
}

fn is_embeddable_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| EMBEDDABLE_EXTENSIONS.contains(&ext))
}

fn is_native_addon_specifier(specifier: &str) -> bool {
    specifier.ends_with(".node")
        || Path::new(specifier)
            .extension()
            .and_then(|ext| ext.to_str())
            == Some("node")
}

fn ensure_embeddable(path: &Path) -> Result<()> {
    if path.is_file() && is_embeddable_file(path) {
        Ok(())
    } else {
        Err(unsupported_module(path))
    }
}

fn classify_specifier(from: &Path, specifier: &str) -> Result<Option<PathBuf>> {
    if is_native_addon_specifier(specifier) {
        return Err(compile_error(format!(
            "native addon '{specifier}' is not embedded (imported by {})",
            from.display()
        )));
    }
    if Path::new(specifier).is_absolute() {
        return Err(compile_error(format!(
            "absolute module specifiers are not embedded ('{specifier}' imported by {})",
            from.display()
        )));
    }
    if specifier.starts_with('.') {
        let Some(resolved) = crate::tooling::bundler::resolve_module_path(from, specifier) else {
            return Err(compile_error(format!(
                "cannot resolve '{specifier}' from {}",
                from.display()
            )));
        };
        if is_native_addon_specifier(&resolved.to_string_lossy()) {
            return Err(compile_error(format!(
                "native addon '{specifier}' is not embedded (imported by {})",
                from.display()
            )));
        }
        ensure_embeddable(&resolved)?;
        return Ok(Some(resolved));
    }
    if let Some(resolved) = crate::tooling::bundler::resolve_module_path(from, specifier) {
        if is_native_addon_specifier(&resolved.to_string_lossy()) {
            return Err(compile_error(format!(
                "native addon '{specifier}' is not embedded (imported by {})",
                from.display()
            )));
        }
        ensure_embeddable(&resolved)?;
        return Ok(Some(resolved));
    }
    Ok(None)
}

struct DynamicLoadCheck {
    dynamic_import: bool,
    computed_require: bool,
}

impl<'a> Visit<'a> for DynamicLoadCheck {
    fn enter_node(&mut self, kind: oxc::ast::AstKind<'a>) {
        if self.dynamic_import || self.computed_require {
            return;
        }
        match kind {
            oxc::ast::AstKind::ImportExpression(_) => {
                self.dynamic_import = true;
            }
            oxc::ast::AstKind::CallExpression(call) => {
                if is_direct_require(call) && !require_arg_is_string_literal(call) {
                    self.computed_require = true;
                }
            }
            _ => {}
        }
    }
}

fn is_direct_require(call: &CallExpression<'_>) -> bool {
    match &call.callee {
        Expression::Identifier(ident) => ident.name == "require",
        _ => false,
    }
}

fn require_arg_is_string_literal(call: &CallExpression<'_>) -> bool {
    matches!(call.arguments.first(), Some(Argument::StringLiteral(_)))
}

fn reject_dynamic_loads(path: &Path, source: &str) -> Result<()> {
    if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        return Ok(());
    }
    let source_type = SourceType::from_path(path)
        .map_err(|err| compile_error(format!("unsupported module '{}': {err}", path.display())))?;
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let mut check = DynamicLoadCheck {
        dynamic_import: false,
        computed_require: false,
    };
    check.visit_program(&parsed.program);
    if check.dynamic_import {
        return Err(compile_error(format!(
            "dynamic import() is not embedded (in {})",
            path.display()
        )));
    }
    if check.computed_require {
        return Err(compile_error(format!(
            "computed require() is not embedded (in {})",
            path.display()
        )));
    }
    Ok(())
}

fn preflight_embeddable(entry: &Path) -> Result<()> {
    if !entry.exists() {
        return Err(compile_error(format!(
            "entry file not found: {}",
            entry.display()
        )));
    }
    if !entry.is_file() {
        return Err(compile_error(format!(
            "entry must be a file: {}",
            entry.display()
        )));
    }
    ensure_embeddable(entry)?;

    let mut visited = HashSet::new();
    let mut queue = vec![entry.to_path_buf()];
    while let Some(current) = queue.pop() {
        let key = current.canonicalize().unwrap_or_else(|_| current.clone());
        if !visited.insert(key) {
            continue;
        }
        let source = fs::read_to_string(&current).map_err(|err| {
            compile_error(format!("failed to read '{}': {err}", current.display()))
        })?;
        reject_dynamic_loads(&current, &source)?;
        if current.extension().and_then(|ext| ext.to_str()) == Some("json") {
            continue;
        }
        for specifier in crate::tooling::bundler::scan_import_specifiers(&source) {
            if let Some(next) = classify_specifier(&current, &specifier)? {
                queue.push(next);
            }
        }
    }
    Ok(())
}

fn same_file(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn embed_payload(runtime_binary: &Path, output_path: &Path, payload: &str) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|err| {
                compile_error(format!(
                    "failed to create output directory '{}': {err}",
                    parent.display()
                ))
            })?;
        }
    }

    fs::copy(runtime_binary, output_path).map_err(|err| {
        compile_error(format!(
            "failed to create binary at '{}': {err}",
            output_path.display()
        ))
    })?;

    let write_result = write_trailer(output_path, payload);
    if let Err(err) = write_result {
        let _ = fs::remove_file(output_path);
        return Err(err);
    }
    Ok(())
}

const MH_MAGIC_64: u32 = 0xfeed_facf;
const LC_SEGMENT_64: u32 = 0x19;
const SEG_CMD_SIZE: usize = 72;

// Used when embedding a SEA payload into a Mach-O (`target_os = "macos"`)
// and by the layout tests. Linux/Windows release builds do not call them.
#[cfg(any(target_os = "macos", test))]
const FAT_MAGIC: u32 = 0xcafe_babe;
#[cfg(any(target_os = "macos", test))]
const FAT_CIGAM: u32 = 0xbeba_feca;
#[cfg(any(target_os = "macos", test))]
const FAT_MAGIC_64: u32 = 0xcafe_babf;
#[cfg(any(target_os = "macos", test))]
const CPU_TYPE_X86_64: u32 = 0x0100_0007;
#[cfg(any(target_os = "macos", test))]
const CPU_TYPE_ARM64: u32 = 0x0100_000c;
#[cfg(any(target_os = "macos", test))]
const LC_SYMTAB: u32 = 0x2;
#[cfg(any(target_os = "macos", test))]
const LC_DYSYMTAB: u32 = 0xb;
#[cfg(any(target_os = "macos", test))]
const LC_TWOLEVEL_HINTS: u32 = 0x16;
#[cfg(any(target_os = "macos", test))]
const LC_DYLD_INFO: u32 = 0x22;
#[cfg(any(target_os = "macos", test))]
const LC_DYLD_INFO_ONLY: u32 = 0x8000_0022;
#[cfg(any(target_os = "macos", test))]
const LC_ENCRYPTION_INFO_64: u32 = 0x2c;
#[cfg(any(target_os = "macos", test))]
const SECT_SIZE: usize = 80;
#[cfg(any(target_os = "macos", test))]
const AMBER_CMD_SIZE: usize = SEG_CMD_SIZE + SECT_SIZE;

struct AmberSegment {
    start: usize,
    end: usize,
}

#[cfg(any(target_os = "macos", test))]
fn macho_page_size(cputype: u32) -> Result<u64> {
    match cputype {
        CPU_TYPE_ARM64 => Ok(0x4000),
        CPU_TYPE_X86_64 => Ok(0x1000),
        other => Err(compile_error(format!(
            "unsupported Mach-O cputype {other:#x} (amber compile signs the host binary only)"
        ))),
    }
}

#[cfg(any(target_os = "macos", test))]
fn align_up(value: u64, page: u64) -> u64 {
    let rem = value % page;
    if rem == 0 {
        value
    } else {
        value + (page - rem)
    }
}

fn segname_is(bytes: &[u8], name: &str) -> bool {
    let mut padded = [0u8; 16];
    padded[..name.len()].copy_from_slice(name.as_bytes());
    bytes == padded
}

#[cfg(any(target_os = "macos", test))]
fn write_padded_name(dest: &mut [u8], name: &str) {
    dest[..name.len()].copy_from_slice(name.as_bytes());
}

#[cfg(any(target_os = "macos", test))]
fn is_linkedit_data_cmd(cmd: u32) -> bool {
    matches!(
        cmd,
        0x1d | 0x1e | 0x26 | 0x29 | 0x2b | 0x2e | 0x36 | 0x37 | 0x38 | 0x8000_0033 | 0x8000_0034
    )
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32> {
    let end = at
        .checked_add(4)
        .ok_or_else(|| compile_error("Mach-O offset overflow"))?;
    let slice = bytes
        .get(at..end)
        .ok_or_else(|| compile_error("Mach-O load command is truncated"))?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

fn read_u64(bytes: &[u8], at: usize) -> Result<u64> {
    let end = at
        .checked_add(8)
        .ok_or_else(|| compile_error("Mach-O offset overflow"))?;
    let slice = bytes
        .get(at..end)
        .ok_or_else(|| compile_error("Mach-O load command is truncated"))?;
    Ok(u64::from_le_bytes(slice.try_into().unwrap()))
}

#[cfg(any(target_os = "macos", test))]
fn bump_u32(buf: &mut [u8], at: usize, threshold: u64, span: u64) -> Result<()> {
    let value = read_u32(buf, at)? as u64;
    if value == 0 || value < threshold {
        return Ok(());
    }
    let bumped = value + span;
    let bumped_u32 = u32::try_from(bumped)
        .map_err(|_| compile_error("Mach-O file offset does not fit in u32 after SEA insert"))?;
    buf[at..at + 4].copy_from_slice(&bumped_u32.to_le_bytes());
    Ok(())
}

#[cfg(any(target_os = "macos", test))]
fn bump_u64_fileoff(buf: &mut [u8], at: usize, threshold: u64, span: u64) -> Result<()> {
    let value = read_u64(buf, at)?;
    if value < threshold {
        return Ok(());
    }
    buf[at..at + 8].copy_from_slice(&(value + span).to_le_bytes());
    Ok(())
}

/// Inserts `blob` (payload + trailer) as segment `__AMBER` immediately before
/// `__LINKEDIT`, shifting that segment down by one page-rounded span.
///
/// The magic stays at the end of `__AMBER`'s file size. `__LINKEDIT` remains
/// the last segment so `codesign` can append an ad-hoc signature.
#[cfg(any(target_os = "macos", test))]
fn inject_amber_segment(macho: &[u8], blob: &[u8]) -> Result<Vec<u8>> {
    if macho.len() < 32 {
        return Err(compile_error("host runtime is not a 64-bit Mach-O"));
    }
    let magic = u32::from_le_bytes(macho[0..4].try_into().unwrap());
    if matches!(magic.swap_bytes(), FAT_MAGIC | FAT_MAGIC_64)
        || matches!(magic, FAT_MAGIC | FAT_CIGAM | FAT_MAGIC_64)
    {
        return Err(compile_error(
            "universal Mach-O is not supported (amber compile does not rewrite fat binaries)",
        ));
    }
    if magic != MH_MAGIC_64 {
        return Err(compile_error("host runtime is not a 64-bit Mach-O"));
    }
    if blob.is_empty() {
        return Err(compile_error("bundle produced an empty payload"));
    }

    let cputype = read_u32(macho, 4)?;
    let page = macho_page_size(cputype)?;
    let ncmds = read_u32(macho, 16)? as usize;
    let sizeofcmds = read_u32(macho, 20)? as usize;
    let header_end = 32 + sizeofcmds;
    if ncmds == 0 || sizeofcmds == 0 || header_end > macho.len() {
        return Err(compile_error("Mach-O load commands are truncated"));
    }

    let mut offset = 32usize;
    let mut linkedit_cmd_off = None;
    let mut linkedit_fileoff = None;
    let mut linkedit_vmaddr = None;
    let mut earliest_content: Option<u64> = None;
    let mut other_vm: Vec<(u64, u64)> = Vec::new();

    for _ in 0..ncmds {
        if offset + 8 > header_end {
            return Err(compile_error("Mach-O load command overruns the header"));
        }
        let cmd = read_u32(macho, offset)?;
        let cmdsize = read_u32(macho, offset + 4)? as usize;
        if cmdsize < 8 || offset + cmdsize > header_end {
            return Err(compile_error("Mach-O load command size is invalid"));
        }
        if cmd == LC_SEGMENT_64 {
            if cmdsize < SEG_CMD_SIZE {
                return Err(compile_error("LC_SEGMENT_64 is truncated"));
            }
            let name = &macho[offset + 8..offset + 24];
            let vmaddr = read_u64(macho, offset + 24)?;
            let vmsize = read_u64(macho, offset + 32)?;
            let fileoff = read_u64(macho, offset + 40)?;
            let filesize = read_u64(macho, offset + 48)?;
            let nsects = read_u32(macho, offset + 64)? as usize;
            if SEG_CMD_SIZE + nsects * SECT_SIZE > cmdsize {
                return Err(compile_error("LC_SEGMENT_64 sections overrun the command"));
            }
            if segname_is(name, "__LINKEDIT") {
                if linkedit_cmd_off.is_some() {
                    return Err(compile_error("Mach-O has more than one __LINKEDIT segment"));
                }
                linkedit_cmd_off = Some(offset);
                linkedit_fileoff = Some(fileoff);
                linkedit_vmaddr = Some(vmaddr);
            } else if segname_is(name, "__AMBER") {
                return Err(compile_error(
                    "host runtime already contains an __AMBER segment",
                ));
            } else {
                if filesize > 0 && fileoff > 0 {
                    earliest_content =
                        Some(earliest_content.map_or(fileoff, |earliest| earliest.min(fileoff)));
                }
                if vmsize > 0 {
                    other_vm.push((vmaddr, vmaddr.saturating_add(vmsize)));
                }
            }
            let mut sect = offset + SEG_CMD_SIZE;
            for _ in 0..nsects {
                let sect_size = read_u64(macho, sect + 40)?;
                let sect_off = read_u32(macho, sect + 48)? as u64;
                if sect_size > 0 && sect_off > 0 {
                    earliest_content =
                        Some(earliest_content.map_or(sect_off, |earliest| earliest.min(sect_off)));
                }
                sect += SECT_SIZE;
            }
        }
        offset += cmdsize;
    }
    if offset != header_end {
        return Err(compile_error("Mach-O load commands do not fill sizeofcmds"));
    }

    let linkedit_cmd_off =
        linkedit_cmd_off.ok_or_else(|| compile_error("Mach-O is missing __LINKEDIT"))?;
    let linkedit_fileoff = linkedit_fileoff.unwrap();
    let linkedit_vmaddr = linkedit_vmaddr.unwrap();
    if linkedit_fileoff > macho.len() as u64 {
        return Err(compile_error("__LINKEDIT starts past the end of the file"));
    }
    if linkedit_fileoff % page != 0 || linkedit_vmaddr % page != 0 {
        return Err(compile_error(
            "__LINKEDIT is not page-aligned (cannot insert __AMBER before it)",
        ));
    }

    offset = 32;
    for _ in 0..ncmds {
        let cmd = read_u32(macho, offset)?;
        let cmdsize = read_u32(macho, offset + 4)? as usize;
        if cmd == LC_SEGMENT_64 {
            let fileoff = read_u64(macho, offset + 40)?;
            let filesize = read_u64(macho, offset + 48)?;
            let name = &macho[offset + 8..offset + 24];
            if filesize > 0 && fileoff > linkedit_fileoff && !segname_is(name, "__LINKEDIT") {
                return Err(compile_error(
                    "Mach-O has a segment after __LINKEDIT; amber compile cannot sign that layout",
                ));
            }
        }
        offset += cmdsize;
    }

    let earliest = earliest_content.unwrap_or(linkedit_fileoff);
    if earliest < header_end as u64 + AMBER_CMD_SIZE as u64 {
        return Err(compile_error(
            "Mach-O header has no room for the __AMBER segment",
        ));
    }
    let blob_len = blob.len() as u64;
    let span = align_up(blob_len, page);
    let amber_vm_end = linkedit_vmaddr.saturating_add(span);
    for (start, end) in other_vm {
        if start < amber_vm_end && linkedit_vmaddr < end {
            return Err(compile_error(
                "Mach-O VM layout has no gap for the __AMBER segment",
            ));
        }
    }

    let mut new_cmd = vec![0u8; AMBER_CMD_SIZE];
    new_cmd[0..4].copy_from_slice(&LC_SEGMENT_64.to_le_bytes());
    new_cmd[4..8].copy_from_slice(&(AMBER_CMD_SIZE as u32).to_le_bytes());
    write_padded_name(&mut new_cmd[8..24], "__AMBER");
    new_cmd[24..32].copy_from_slice(&linkedit_vmaddr.to_le_bytes());
    new_cmd[32..40].copy_from_slice(&span.to_le_bytes());
    new_cmd[40..48].copy_from_slice(&linkedit_fileoff.to_le_bytes());
    new_cmd[48..56].copy_from_slice(&blob_len.to_le_bytes());
    new_cmd[56..60].copy_from_slice(&1i32.to_le_bytes());
    new_cmd[60..64].copy_from_slice(&1i32.to_le_bytes());
    new_cmd[64..68].copy_from_slice(&1u32.to_le_bytes());
    write_padded_name(&mut new_cmd[72..88], "__payload");
    write_padded_name(&mut new_cmd[88..104], "__AMBER");
    new_cmd[104..112].copy_from_slice(&linkedit_vmaddr.to_le_bytes());
    new_cmd[112..120].copy_from_slice(&blob_len.to_le_bytes());
    let fileoff_u32 = u32::try_from(linkedit_fileoff)
        .map_err(|_| compile_error("Mach-O file offset does not fit in u32"))?;
    new_cmd[120..124].copy_from_slice(&fileoff_u32.to_le_bytes());
    new_cmd[124..128].copy_from_slice(&3u32.to_le_bytes());

    let linkedit_off_us = usize::try_from(linkedit_fileoff)
        .map_err(|_| compile_error("Mach-O file offset does not fit in usize"))?;
    let new_header_end = header_end + AMBER_CMD_SIZE;
    if linkedit_off_us < new_header_end {
        return Err(compile_error("Mach-O header overlaps __LINKEDIT"));
    }

    let mut out = Vec::with_capacity(macho.len() + span as usize);
    out.extend_from_slice(&macho[..linkedit_cmd_off]);
    out.extend_from_slice(&new_cmd);
    out.extend_from_slice(&macho[linkedit_cmd_off..header_end]);
    out.extend_from_slice(&macho[new_header_end..linkedit_off_us]);
    out.extend_from_slice(blob);
    out.resize(linkedit_off_us + span as usize, 0);
    out.extend_from_slice(&macho[linkedit_off_us..]);

    let ncmds_new = (ncmds as u32) + 1;
    let sizeofcmds_new = (sizeofcmds as u32) + AMBER_CMD_SIZE as u32;
    out[16..20].copy_from_slice(&ncmds_new.to_le_bytes());
    out[20..24].copy_from_slice(&sizeofcmds_new.to_le_bytes());

    bump_linkedit_offsets(
        &mut out,
        ncmds_new as usize,
        sizeofcmds_new as usize,
        linkedit_fileoff,
        span,
    )?;
    Ok(out)
}

#[cfg(any(target_os = "macos", test))]
fn bump_linkedit_offsets(
    buf: &mut [u8],
    ncmds: usize,
    sizeofcmds: usize,
    threshold: u64,
    span: u64,
) -> Result<()> {
    let header_end = 32 + sizeofcmds;
    let mut offset = 32usize;
    for _ in 0..ncmds {
        if offset + 8 > header_end || offset + 8 > buf.len() {
            return Err(compile_error(
                "Mach-O load command overruns the header after SEA insert",
            ));
        }
        let cmd = read_u32(buf, offset)?;
        let cmdsize = read_u32(buf, offset + 4)? as usize;
        if cmdsize < 8 || offset + cmdsize > header_end {
            return Err(compile_error(
                "Mach-O load command size is invalid after SEA insert",
            ));
        }
        if cmd == LC_SEGMENT_64 {
            let is_amber = segname_is(&buf[offset + 8..offset + 24], "__AMBER");
            let is_linkedit = segname_is(&buf[offset + 8..offset + 24], "__LINKEDIT");
            if !is_amber {
                bump_u64_fileoff(buf, offset + 40, threshold, span)?;
                if is_linkedit {
                    let vmaddr = read_u64(buf, offset + 24)?;
                    buf[offset + 24..offset + 32].copy_from_slice(&(vmaddr + span).to_le_bytes());
                }
                let nsects = read_u32(buf, offset + 64)? as usize;
                let mut sect = offset + SEG_CMD_SIZE;
                for _ in 0..nsects {
                    bump_u32(buf, sect + 48, threshold, span)?;
                    bump_u32(buf, sect + 56, threshold, span)?;
                    sect += SECT_SIZE;
                }
            }
        } else if cmd == LC_SYMTAB && cmdsize >= 24 {
            bump_u32(buf, offset + 8, threshold, span)?;
            bump_u32(buf, offset + 16, threshold, span)?;
        } else if cmd == LC_DYSYMTAB && cmdsize >= 80 {
            for field in [32, 40, 48, 56, 64, 72] {
                bump_u32(buf, offset + field, threshold, span)?;
            }
        } else if matches!(cmd, LC_DYLD_INFO | LC_DYLD_INFO_ONLY) && cmdsize >= 48 {
            for field in [8, 16, 24, 32, 40] {
                bump_u32(buf, offset + field, threshold, span)?;
            }
        } else if is_linkedit_data_cmd(cmd) && cmdsize >= 16 {
            bump_u32(buf, offset + 8, threshold, span)?;
        } else if cmd == LC_ENCRYPTION_INFO_64 && cmdsize >= 16 {
            bump_u32(buf, offset + 8, threshold, span)?;
        } else if cmd == LC_TWOLEVEL_HINTS && cmdsize >= 16 {
            bump_u32(buf, offset + 8, threshold, span)?;
        }
        offset += cmdsize;
    }
    if offset != header_end {
        return Err(compile_error(
            "Mach-O load commands do not fill sizeofcmds after SEA insert",
        ));
    }
    Ok(())
}

fn macho_amber_segment(bytes: &[u8]) -> Option<AmberSegment> {
    if bytes.len() < 32 {
        return None;
    }
    let magic = u32::from_le_bytes(bytes[0..4].try_into().ok()?);
    if magic != MH_MAGIC_64 {
        return None;
    }
    let ncmds = read_u32(bytes, 16).ok()? as usize;
    let sizeofcmds = read_u32(bytes, 20).ok()? as usize;
    let header_end = 32 + sizeofcmds;
    if header_end > bytes.len() {
        return None;
    }
    let mut offset = 32usize;
    for _ in 0..ncmds {
        if offset + 8 > header_end {
            return None;
        }
        let cmd = read_u32(bytes, offset).ok()?;
        let cmdsize = read_u32(bytes, offset + 4).ok()? as usize;
        if cmdsize < 8 || offset + cmdsize > header_end {
            return None;
        }
        if cmd == LC_SEGMENT_64 && cmdsize >= SEG_CMD_SIZE {
            let name = &bytes[offset + 8..offset + 24];
            if segname_is(name, "__AMBER") {
                let fileoff = read_u64(bytes, offset + 40).ok()?;
                let filesize = read_u64(bytes, offset + 48).ok()?;
                let start = usize::try_from(fileoff).ok()?;
                let size = usize::try_from(filesize).ok()?;
                let end = start.checked_add(size)?;
                if end > bytes.len() {
                    return None;
                }
                return Some(AmberSegment { start, end });
            }
        }
        offset += cmdsize;
    }
    None
}

fn trailer_blob(payload: &str) -> Result<Vec<u8>> {
    let payload_bytes = payload.as_bytes();
    if payload_bytes.is_empty() {
        return Err(compile_error("bundle produced an empty payload"));
    }
    let payload_len = u64::try_from(payload_bytes.len())
        .map_err(|_| compile_error("bundle payload is too large"))?;
    let mut blob = Vec::with_capacity(payload_bytes.len() + TRAILER_TOTAL_SIZE as usize);
    blob.extend_from_slice(payload_bytes);
    blob.extend_from_slice(&payload_len.to_le_bytes());
    blob.extend_from_slice(&0u64.to_le_bytes());
    blob.extend_from_slice(MAGIC_TRAILER);
    Ok(blob)
}

fn append_trailer_bytes(output_path: &Path, blob: &[u8]) -> Result<()> {
    let mut out_file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(output_path)
        .map_err(|err| {
            compile_error(format!(
                "failed to write trailer at '{}': {err}",
                output_path.display()
            ))
        })?;
    out_file
        .write_all(blob)
        .and_then(|_| out_file.flush())
        .map_err(|err| {
            compile_error(format!(
                "failed to write trailer at '{}': {err}",
                output_path.display()
            ))
        })?;
    Ok(())
}

fn write_trailer(output_path: &Path, payload: &str) -> Result<()> {
    let blob = trailer_blob(payload)?;
    // macOS rejects an appended trailer (`main executable failed strict
    // validation`). The payload goes in a segment before `__LINKEDIT`, then
    // `codesign` appends an ad-hoc signature that must be the last bytes.
    #[cfg(target_os = "macos")]
    {
        remove_macos_signature(output_path)?;
        let bytes = fs::read(output_path).map_err(|err| {
            compile_error(format!(
                "failed to read host binary '{}': {err}",
                output_path.display()
            ))
        })?;
        let injected = inject_amber_segment(&bytes, &blob)?;
        fs::write(output_path, injected).map_err(|err| {
            compile_error(format!(
                "failed to write trailer at '{}': {err}",
                output_path.display()
            ))
        })?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        append_trailer_bytes(output_path, &blob)?;
    }

    mark_executable(output_path)?;

    #[cfg(target_os = "macos")]
    sign_macos_adhoc(output_path)?;

    match read_standalone_payload(output_path) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(compile_error(format!(
            "internal error: '{}' has no readable AMBER_STANDALONE trailer after write",
            output_path.display()
        ))),
        Err(err) => Err(compile_error(format!(
            "internal error: trailer check failed for '{}': {err}",
            output_path.display()
        ))),
    }
}

fn mark_executable(output_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(output_path).map_err(|err| {
            compile_error(format!(
                "failed to mark '{}' executable: {err}",
                output_path.display()
            ))
        })?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(output_path, perms).map_err(|err| {
            compile_error(format!(
                "failed to mark '{}' executable: {err}",
                output_path.display()
            ))
        })?;
    }
    #[cfg(not(unix))]
    {
        let _ = output_path;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn sign_macos_adhoc(output_path: &Path) -> Result<()> {
    let output = std::process::Command::new("codesign")
        .args(["--sign", "-", "--force"])
        .arg(output_path)
        .output()
        .map_err(|err| {
            compile_error(format!(
                "macOS codesign failed to start for '{}': {err}",
                output_path.display()
            ))
        })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = stderr.trim();
    let detail = if detail.is_empty() {
        format!("exit status {}", output.status)
    } else {
        detail.to_string()
    };
    Err(compile_error(format!(
        "macOS codesign failed for '{}': {detail}",
        output_path.display()
    )))
}

/// Compiles an entry JS/TS script into a standalone self-executing binary.
pub fn compile_binary(entry_file: &Path, output_path: &Path) -> Result<()> {
    ensure_supported_host()?;
    preflight_embeddable(entry_file)?;

    let runtime_binary = resolve_runtime_binary()?;
    if output_path.is_dir() {
        return Err(compile_error(format!(
            "output path is a directory: {}",
            output_path.display()
        )));
    }
    if same_file(output_path, &runtime_binary) {
        return Err(compile_error(format!(
            "refusing to overwrite the Amber runtime binary: {}",
            output_path.display()
        )));
    }

    let bundle_opts = crate::tooling::bundler::BundleOptions {
        entry: entry_file.to_path_buf(),
        outfile: None,
        minify: false,
        sourcemap: false,
        target: "es2022".to_string(),
        import_map: None,
    };
    let bundle_out = crate::tooling::bundler::bundle_project(&bundle_opts)
        .map_err(|err| compile_error(format!("bundle failed: {err}")))?;

    embed_payload(&runtime_binary, output_path, &bundle_out.code)?;
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

    fn write_trailer_bytes(path: &Path, payload: &[u8], flags: u64) {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(path)
            .expect("open");
        let len = payload.len() as u64;
        file.write_all(payload).expect("payload");
        file.write_all(&len.to_le_bytes()).expect("len");
        file.write_all(&flags.to_le_bytes()).expect("flags");
        file.write_all(MAGIC_TRAILER).expect("magic");
        file.flush().expect("flush");
    }

    #[test]
    fn test_standalone_embed_and_extract() {
        let dir = tempdir().expect("tempdir");
        let fake_exe = dir.path().join("fake_bin");
        fs::write(&fake_exe, b"BASE_BINARY_BYTES").expect("write");
        let script = "console.log('Hello Standalone');";
        write_trailer_bytes(&fake_exe, script.as_bytes(), 0);

        let extracted = read_standalone_payload(&fake_exe)
            .expect("read")
            .expect("payload");
        assert_eq!(extracted, script);
    }

    #[test]
    fn trailer_without_magic_is_not_a_sea_binary() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("plain");
        fs::write(&path, vec![0u8; 64]).expect("write");
        assert!(read_standalone_payload(&path).expect("read").is_none());
    }

    #[test]
    fn short_file_is_not_a_sea_binary() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("short");
        fs::write(&path, b"tiny").expect("write");
        assert!(read_standalone_payload(&path).expect("read").is_none());
    }

    #[test]
    fn nonzero_flags_fail_closed() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("flagged");
        fs::write(&path, b"BASE").expect("write");
        write_trailer_bytes(&path, b"console.log(1)", 1);
        let err = read_standalone_payload(&path)
            .expect_err("flags")
            .to_string();
        assert!(err.starts_with(STANDALONE_ERROR_PREFIX), "{err}");
        assert!(err.contains("unsupported trailer flags: 1"), "{err}");
    }

    #[test]
    fn zero_payload_length_fails_closed() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("empty-len");
        fs::write(&path, b"BASE").expect("write");
        write_trailer_bytes(&path, b"", 0);
        let err = read_standalone_payload(&path).expect_err("len").to_string();
        assert!(err.starts_with(STANDALONE_ERROR_PREFIX), "{err}");
        assert!(err.contains("invalid payload length"), "{err}");
    }

    #[test]
    fn oversized_payload_length_fails_closed() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("huge");
        fs::write(&path, b"BASE").expect("write");
        write_trailer_bytes(&path, b"console.log(1)", 0);
        let mut bytes = fs::read(&path).expect("read");
        let n = bytes.len();
        bytes[n - 32..n - 24].copy_from_slice(&u64::MAX.to_le_bytes());
        fs::write(&path, bytes).expect("rewrite");
        let err = read_standalone_payload(&path).expect_err("len").to_string();
        assert!(err.starts_with(STANDALONE_ERROR_PREFIX), "{err}");
        assert!(err.contains("invalid payload length"), "{err}");
    }

    #[test]
    fn non_utf8_payload_fails_closed() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("binary");
        fs::write(&path, b"BASE").expect("write");
        write_trailer_bytes(&path, &[0xFF, 0xFE, 0xFD], 0);
        let err = read_standalone_payload(&path)
            .expect_err("utf8")
            .to_string();
        assert!(err.starts_with(STANDALONE_ERROR_PREFIX), "{err}");
        assert!(err.contains("payload is not UTF-8"), "{err}");
    }

    fn put_u32(buf: &mut [u8], at: usize, value: u32) {
        buf[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(buf: &mut [u8], at: usize, value: u64) {
        buf[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_seg(
        buf: &mut [u8],
        name: &str,
        vmaddr: u64,
        vmsize: u64,
        fileoff: u64,
        filesize: u64,
    ) {
        put_u32(buf, 0, LC_SEGMENT_64);
        put_u32(buf, 4, SEG_CMD_SIZE as u32);
        write_padded_name(&mut buf[8..24], name);
        put_u64(buf, 24, vmaddr);
        put_u64(buf, 32, vmsize);
        put_u64(buf, 40, fileoff);
        put_u64(buf, 48, filesize);
        put_u32(buf, 56, 1);
        put_u32(buf, 60, 1);
    }

    /// Tiny arm64 Mach-O: `__TEXT` (header + padding) and `__LINKEDIT` with a
    /// symtab pointing at the linkedit bytes.
    fn minimal_macho(linkedit: &[u8]) -> Vec<u8> {
        let page = 0x4000u64;
        let ncmds = 3u32;
        let sizeofcmds = (SEG_CMD_SIZE * 2 + 24) as u32;
        let mut buf = vec![0u8; page as usize + linkedit.len()];
        put_u32(&mut buf, 0, MH_MAGIC_64);
        put_u32(&mut buf, 4, CPU_TYPE_ARM64);
        put_u32(&mut buf, 12, 2);
        put_u32(&mut buf, 16, ncmds);
        put_u32(&mut buf, 20, sizeofcmds);
        write_seg(&mut buf[32..], "__TEXT", 0, page, 0, page);
        write_seg(
            &mut buf[32 + SEG_CMD_SIZE..],
            "__LINKEDIT",
            page,
            page,
            page,
            linkedit.len() as u64,
        );
        let sym = 32 + SEG_CMD_SIZE * 2;
        put_u32(&mut buf, sym, LC_SYMTAB);
        put_u32(&mut buf, sym + 4, 24);
        put_u32(&mut buf, sym + 8, page as u32);
        put_u32(&mut buf, sym + 12, 1);
        put_u32(&mut buf, sym + 16, page as u32 + 8);
        put_u32(&mut buf, sym + 20, 8);
        buf[page as usize..].copy_from_slice(linkedit);
        buf
    }

    #[test]
    fn macho_segment_roundtrip_keeps_trailer_before_linkedit() {
        let linkedit = vec![0xABu8; 32];
        let macho = minimal_macho(&linkedit);
        let script = "console.log('sea')";
        let blob = trailer_blob(script).expect("blob");
        let injected = inject_amber_segment(&macho, &blob).expect("inject");

        assert!(
            !injected.ends_with(MAGIC_TRAILER),
            "macOS trailer is not at EOF; __LINKEDIT follows it"
        );
        let trailer = find_standalone_trailer(&injected)
            .expect("locate")
            .expect("trailer");
        assert_eq!(trailer.flags, 0);
        let payload = std::str::from_utf8(
            &injected[trailer.payload_start..trailer.payload_start + trailer.payload_len as usize],
        )
        .expect("utf8");
        assert_eq!(payload, script);

        let linkedit_at = 0x4000 + 0x4000;
        assert_eq!(
            &injected[linkedit_at..linkedit_at + 32],
            linkedit.as_slice()
        );
        let sym = 32 + AMBER_CMD_SIZE + SEG_CMD_SIZE * 2;
        let symoff = u32::from_le_bytes(injected[sym + 8..sym + 12].try_into().unwrap());
        assert_eq!(symoff, linkedit_at as u32);
        assert_eq!(read_u32(&injected, 16).unwrap(), 4);
    }

    #[test]
    fn macho_without_amber_segment_is_not_a_sea_binary() {
        let macho = minimal_macho(&[0u8; 16]);
        assert!(find_standalone_trailer(&macho).expect("read").is_none());
    }

    #[test]
    fn macho_header_without_padding_is_rejected() {
        let mut macho = minimal_macho(&[1u8; 8]);
        let header_end = 32 + SEG_CMD_SIZE * 2 + 24;
        put_u64(&mut macho, 32 + 48, header_end as u64);
        put_u64(&mut macho, 32 + SEG_CMD_SIZE + 40, header_end as u64);
        put_u64(&mut macho, 32 + SEG_CMD_SIZE + 24, header_end as u64);
        macho.truncate(header_end + 8);
        let err = inject_amber_segment(&macho, b"payload-and-not-a-real-trailer")
            .expect_err("no room")
            .to_string();
        assert!(
            err.contains("no room") || err.contains("not page-aligned") || err.contains("overlaps"),
            "{err}"
        );
    }
}
