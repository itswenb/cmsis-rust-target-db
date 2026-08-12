mod model;
mod output;
mod pdsc;
mod target;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use walkdir::WalkDir;

use crate::model::DeviceRecord;

#[derive(Debug, Parser)]
#[command(name = "cmsis-rust-target-db")]
#[command(about = "Generate/query MCU Rust target metadata from CMSIS-Pack PDSC files")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Generate {
        #[arg(long)]
        pdsc_root: PathBuf,
        #[arg(long, default_value = "data")]
        out_dir: PathBuf,
        #[arg(long)]
        index_file: Option<PathBuf>,
    },
    Query {
        device: String,
        #[arg(long, default_value = "data/devices.jsonl")]
        data: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate {
            pdsc_root,
            out_dir,
            index_file,
        } => generate(&pdsc_root, &out_dir, index_file.as_deref()),
        Command::Query { device, data } => query(&device, &data),
    }
}

fn generate(pdsc_root: &Path, out_dir: &Path, index_file: Option<&Path>) -> Result<()> {
    let mut pdsc_files: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(pdsc_root).follow_links(false) {
        let entry =
            entry.with_context(|| format!("遍历 PDSC 目录失败：{}", pdsc_root.display()))?;
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pdsc"))
        {
            pdsc_files.push(entry.into_path());
        }
    }
    pdsc_files.sort();
    if pdsc_files.is_empty() {
        bail!("no PDSC files found under {}", pdsc_root.display());
    }

    let mut records = Vec::new();
    for path in &pdsc_files {
        let mut parsed = pdsc::parse_pdsc(path)
            .with_context(|| format!("while processing {}", path.display()))?;
        records.append(&mut parsed);
    }

    records.sort();
    records.dedup();
    if records.is_empty() {
        bail!("no device records generated from {}", pdsc_root.display());
    }

    output::write_outputs(out_dir, &records, pdsc_files.len(), index_file)?;
    eprintln!(
        "generated {} records from {} PDSC files into {}",
        records.len(),
        pdsc_files.len(),
        out_dir.display()
    );
    Ok(())
}

fn query(device: &str, data: &Path) -> Result<()> {
    let file = File::open(data).with_context(|| format!("failed to open {}", data.display()))?;
    let reader = BufReader::new(file);
    let needle = device.to_ascii_lowercase();
    let mut exact = Vec::new();
    let mut partial = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: DeviceRecord = serde_json::from_str(&line)?;
        let candidate = record.device.to_ascii_lowercase();
        if candidate == needle {
            exact.push(record);
        } else if candidate.contains(&needle) {
            partial.push(record);
        }
    }

    let matches = if exact.is_empty() { partial } else { exact };
    if matches.is_empty() {
        bail!("no device matching {device:?} in {}", data.display());
    }

    for record in matches {
        println!("{}", serde_json::to_string_pretty(&record)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn reports_pdsc_directory_walk_errors() -> Result<()> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cmsis-rust-target-db-missing-{}-{unique}",
            std::process::id()
        ));

        let error = generate(&root, &root.join("out"), None)
            .expect_err("不存在的 PDSC 目录必须返回遍历错误");
        let chain = format!("{error:#}");

        assert!(chain.contains("遍历 PDSC 目录失败"), "{chain}");
        assert!(chain.contains(&root.display().to_string()), "{chain}");
        Ok(())
    }
}
