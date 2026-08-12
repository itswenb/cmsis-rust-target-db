use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::model::{DeviceRecord, Metadata};

pub fn write_outputs(
    out_dir: &Path,
    records: &[DeviceRecord],
    pdsc_file_count: usize,
    index_file: Option<&Path>,
) -> Result<()> {
    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;

    write_jsonl(&out_dir.join("devices.jsonl"), records)?;
    write_csv(&out_dir.join("devices.csv"), records)?;

    let metadata = Metadata {
        schema_version: 1,
        record_count: records.len(),
        pdsc_file_count,
        rust_target_resolved_count: records.iter().filter(|r| r.rust_target.is_some()).count(),
        rust_target_unresolved_count: records.iter().filter(|r| r.rust_target.is_none()).count(),
        source_index_sha256: index_file.map(sha256_file).transpose()?,
    };

    let mut meta = BufWriter::new(File::create(out_dir.join("metadata.json"))?);
    serde_json::to_writer_pretty(&mut meta, &metadata)?;
    writeln!(meta)?;

    Ok(())
}

fn write_jsonl(path: &Path, records: &[DeviceRecord]) -> Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    for record in records {
        serde_json::to_writer(&mut writer, record)?;
        writeln!(writer)?;
    }
    Ok(())
}

fn write_csv(path: &Path, records: &[DeviceRecord]) -> Result<()> {
    let mut writer = csv::WriterBuilder::new().from_path(path)?;
    for record in records {
        writer.serialize(record)?;
    }
    writer.flush()?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read index file {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    const EXPECTED_INDEX_SHA256: &str =
        "1d420e32395235d5a22d5fcbff43d24fc6b7a01cd6b139c9431e1ff7332c8f3e";

    #[test]
    fn writes_valid_deterministic_outputs_and_metadata() -> Result<()> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cmsis-rust-target-db-output-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;

        let index = root.join("index.pidx");
        fs::write(&index, b"index fixture\n")?;

        let resolved = DeviceRecord {
            vendor: "GigaDevice".into(),
            device: "GD32F303CG".into(),
            device_kind: "device".into(),
            parent_device: None,
            processor: None,
            processor_units: None,
            core: Some("Cortex-M4".into()),
            core_version: None,
            fpu: Some("SP_FPU".into()),
            endian: Some("Little-endian".into()),
            dsp: None,
            mve: None,
            trustzone: None,
            mpu: Some("MPU".into()),
            clock_hz: Some(120_000_000),
            rust_target: Some("thumbv7em-none-eabihf".into()),
            source_pack_vendor: "GigaDevice".into(),
            source_pack_name: "GD32F30x_DFP".into(),
            source_pack_version: Some("2.2.1".into()),
            source_url: Some("https://example.invalid/pack.pdsc".into()),
            source_pdsc: "GigaDevice.GD32F30x_DFP.pdsc".into(),
        };
        let unresolved = DeviceRecord {
            device: "UNKNOWN".into(),
            core: Some("Unknown-Core".into()),
            fpu: None,
            rust_target: None,
            ..resolved.clone()
        };
        let records = [resolved, unresolved];
        let first = root.join("first");
        let second = root.join("second");

        write_outputs(&first, &records, 7, Some(&index))?;
        write_outputs(&second, &records, 7, Some(&index))?;

        let jsonl = fs::read_to_string(first.join("devices.jsonl"))?;
        let json_lines = jsonl.lines().collect::<Vec<_>>();
        assert_eq!(json_lines.len(), 2);
        for line in json_lines {
            serde_json::from_str::<DeviceRecord>(line)?;
        }

        let mut csv = csv::Reader::from_path(first.join("devices.csv"))?;
        assert_eq!(
            csv.headers()?.iter().collect::<Vec<_>>(),
            vec![
                "vendor",
                "device",
                "device_kind",
                "parent_device",
                "processor",
                "processor_units",
                "core",
                "core_version",
                "fpu",
                "endian",
                "dsp",
                "mve",
                "trustzone",
                "mpu",
                "clock_hz",
                "rust_target",
                "source_pack_vendor",
                "source_pack_name",
                "source_pack_version",
                "source_url",
                "source_pdsc",
            ]
        );
        assert_eq!(csv.records().collect::<csv::Result<Vec<_>>>()?.len(), 2);

        let metadata: Metadata = serde_json::from_slice(&fs::read(first.join("metadata.json"))?)?;
        assert_eq!(metadata.record_count, 2);
        assert_eq!(metadata.pdsc_file_count, 7);
        assert_eq!(metadata.rust_target_resolved_count, 1);
        assert_eq!(metadata.rust_target_unresolved_count, 1);
        assert_eq!(
            metadata.source_index_sha256.as_deref(),
            Some(EXPECTED_INDEX_SHA256)
        );

        for name in ["devices.jsonl", "devices.csv", "metadata.json"] {
            assert_eq!(fs::read(first.join(name))?, fs::read(second.join(name))?);
        }

        let _ = fs::remove_dir_all(root);
        Ok(())
    }
}
