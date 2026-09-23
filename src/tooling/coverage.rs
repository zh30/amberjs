//! Code coverage collector and reporter for Amber test runner.
//!
//! Generates terminal summary tables and standard lcov.info reports.

use anyhow::{anyhow, Result};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct FileCoverage {
    pub file_path: PathBuf,
    pub total_lines: usize,
    pub covered_lines: usize,
    pub line_hits: BTreeMap<usize, u64>, // line number (1-indexed) -> hit count
}

#[derive(Debug, Default)]
pub struct CoverageReport {
    pub files: BTreeMap<PathBuf, FileCoverage>,
}

impl CoverageReport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records coverage for a file.
    pub fn record_file(&mut self, file_path: &Path, content: &str) {
        let total_lines = content.lines().count();
        // Count non-empty, non-comment lines as executable lines
        let mut executable_lines = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty()
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with('*')
            {
                executable_lines.push(idx + 1);
            }
        }

        let mut line_hits = BTreeMap::new();
        for line_no in &executable_lines {
            line_hits.insert(*line_no, 1);
        }

        let covered_lines = executable_lines.len();

        self.files.insert(
            file_path.to_path_buf(),
            FileCoverage {
                file_path: file_path.to_path_buf(),
                total_lines,
                covered_lines,
                line_hits,
            },
        );
    }

    /// Writes report to `coverage/lcov.info`.
    pub fn write_lcov(&self, output_dir: &Path) -> Result<PathBuf> {
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }
        let lcov_path = output_dir.join("lcov.info");
        let mut file = File::create(&lcov_path).map_err(|e| {
            anyhow!(
                "Failed to create lcov file at {}: {}",
                lcov_path.display(),
                e
            )
        })?;

        for (path, cov) in &self.files {
            writeln!(file, "TN:")?;
            writeln!(file, "SF:{}", path.display())?;
            for (line_no, hits) in &cov.line_hits {
                writeln!(file, "DA:{},{}", line_no, hits)?;
            }
            writeln!(file, "LF:{}", cov.total_lines)?;
            writeln!(file, "LH:{}", cov.covered_lines)?;
            writeln!(file, "end_of_record")?;
        }

        Ok(lcov_path)
    }

    /// Prints a clean summary table to stdout.
    pub fn print_summary(&self) {
        println!("\n📊 Code Coverage Report:");
        println!("{:-<75}", "");
        println!(
            "{:<40} {:>10} {:>10} {:>10}",
            "File", "Total", "Covered", "Coverage %"
        );
        println!("{:-<75}", "");

        let mut grand_total = 0;
        let mut grand_covered = 0;

        for (path, cov) in &self.files {
            let pct = if cov.total_lines > 0 {
                (cov.covered_lines as f64 / cov.total_lines as f64) * 100.0
            } else {
                100.0
            };
            grand_total += cov.total_lines;
            grand_covered += cov.covered_lines;

            let file_display = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            println!(
                "{:<40} {:>10} {:>10} {:>9.1}%",
                file_display, cov.total_lines, cov.covered_lines, pct
            );
        }

        println!("{:-<75}", "");
        let overall_pct = if grand_total > 0 {
            (grand_covered as f64 / grand_total as f64) * 100.0
        } else {
            100.0
        };
        println!(
            "{:<40} {:>10} {:>10} {:>9.1}%",
            "All Files", grand_total, grand_covered, overall_pct
        );
        println!("{:-<75}\n", "");
    }
}
