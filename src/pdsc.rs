use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use roxmltree::{Document, Node};

use crate::model::DeviceRecord;
use crate::target::rust_target;

#[derive(Debug, Clone, Default)]
struct ProcessorAttrs {
    values: BTreeMap<String, String>,
}

impl ProcessorAttrs {
    fn overlay(&mut self, other: &ProcessorAttrs) {
        for (key, value) in &other.values {
            self.values.insert(key.clone(), value.clone());
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

#[derive(Debug, Clone, Default)]
struct DeviceContext {
    vendor: Option<String>,
    common_processor: ProcessorAttrs,
    named_processors: BTreeMap<String, ProcessorAttrs>,
}

impl DeviceContext {
    fn apply_node(&self, node: Node<'_, '_>) -> Self {
        let mut next = self.clone();

        if let Some(vendor) = node.attribute("Dvendor") {
            next.vendor = Some(normalize_vendor(vendor));
        }

        let processors: Vec<_> = node
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "processor")
            .collect();

        // 没有 Pname 的处理器配置适用于所有处理器，因此较低层级的配置
        // 必须覆盖每个处理器继承的属性。
        for processor in processors.iter().filter(|n| n.attribute("Pname").is_none()) {
            let attrs = processor_attrs(*processor);
            next.common_processor.overlay(&attrs);
            for named in next.named_processors.values_mut() {
                named.overlay(&attrs);
            }
        }

        // 同一层级中，处理器专属配置在通用配置之后应用。
        for processor in processors.iter().filter(|n| n.attribute("Pname").is_some()) {
            let pname = processor.attribute("Pname").unwrap().to_owned();
            let attrs = processor_attrs(*processor);
            let base = next.common_processor.clone();
            let entry = next.named_processors.entry(pname).or_insert(base);
            entry.overlay(&attrs);
        }

        next
    }

    fn effective_processors(&self) -> Vec<(Option<String>, ProcessorAttrs)> {
        if self.named_processors.is_empty() {
            if self.common_processor.values.is_empty() {
                Vec::new()
            } else {
                vec![(None, self.common_processor.clone())]
            }
        } else {
            self.named_processors
                .iter()
                .map(|(name, attrs)| (Some(name.clone()), attrs.clone()))
                .collect()
        }
    }
}

#[derive(Debug, Clone)]
struct PackInfo {
    vendor: String,
    name: String,
    version: Option<String>,
    url: Option<String>,
    pdsc_name: String,
}

pub fn parse_pdsc(path: &Path) -> Result<Vec<DeviceRecord>> {
    let xml =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let doc =
        Document::parse(&xml).with_context(|| format!("failed to parse XML {}", path.display()))?;

    let package = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "package")
        .ok_or_else(|| anyhow!("{} has no <package> element", path.display()))?;

    let pack = PackInfo {
        vendor: direct_child_text(package, "vendor").unwrap_or_else(|| "Unknown".to_owned()),
        name: direct_child_text(package, "name").unwrap_or_else(|| "Unknown".to_owned()),
        version: direct_child(package, "releases")
            .and_then(|r| direct_child(r, "release"))
            .and_then(|r| r.attribute("version"))
            .map(str::to_owned),
        url: direct_child_text(package, "url"),
        pdsc_name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned(),
    };

    let Some(devices) = direct_child(package, "devices") else {
        return Ok(Vec::new());
    };

    let mut records = Vec::new();
    for family in devices
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "family")
    {
        walk_group(family, &DeviceContext::default(), &pack, None, &mut records)?;
    }

    Ok(records)
}

fn walk_group(
    node: Node<'_, '_>,
    parent_ctx: &DeviceContext,
    pack: &PackInfo,
    parent_device: Option<&str>,
    out: &mut Vec<DeviceRecord>,
) -> Result<()> {
    let ctx = parent_ctx.apply_node(node);

    match node.tag_name().name() {
        "family" | "subFamily" => {
            for child in node.children().filter(roxmltree::Node::is_element) {
                if matches!(child.tag_name().name(), "subFamily" | "device") {
                    walk_group(child, &ctx, pack, parent_device, out)?;
                }
            }
        }
        "device" => {
            let name = node
                .attribute("Dname")
                .ok_or_else(|| anyhow!("device without Dname in {}", pack.pdsc_name))?;
            emit_device(name, "device", None, &ctx, pack, out);

            for variant in node
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "variant")
            {
                walk_group(variant, &ctx, pack, Some(name), out)?;
            }
        }
        "variant" => {
            let name = node
                .attribute("Dvariant")
                .ok_or_else(|| anyhow!("variant without Dvariant in {}", pack.pdsc_name))?;
            emit_device(name, "variant", parent_device, &ctx, pack, out);
        }
        _ => {}
    }

    Ok(())
}

fn emit_device(
    name: &str,
    kind: &str,
    parent_device: Option<&str>,
    ctx: &DeviceContext,
    pack: &PackInfo,
    out: &mut Vec<DeviceRecord>,
) {
    let vendor = ctx.vendor.clone().unwrap_or_else(|| pack.vendor.clone());
    let processors = ctx.effective_processors();

    if processors.is_empty() {
        out.push(record_from_processor(
            vendor,
            name,
            kind,
            parent_device,
            None,
            &ProcessorAttrs::default(),
            pack,
        ));
        return;
    }

    for (pname, attrs) in processors {
        out.push(record_from_processor(
            vendor.clone(),
            name,
            kind,
            parent_device,
            pname,
            &attrs,
            pack,
        ));
    }
}

fn record_from_processor(
    vendor: String,
    device: &str,
    device_kind: &str,
    parent_device: Option<&str>,
    processor: Option<String>,
    attrs: &ProcessorAttrs,
    pack: &PackInfo,
) -> DeviceRecord {
    let core = attrs.get("Dcore").map(str::to_owned);
    let fpu = attrs.get("Dfpu").map(normalize_fpu);
    let target = rust_target(core.as_deref(), fpu.as_deref()).map(str::to_owned);

    DeviceRecord {
        vendor,
        device: device.to_owned(),
        device_kind: device_kind.to_owned(),
        parent_device: parent_device.map(str::to_owned),
        processor,
        processor_units: attrs.get("Punits").and_then(|v| v.parse().ok()),
        core,
        core_version: attrs.get("DcoreVersion").map(str::to_owned),
        fpu,
        endian: attrs.get("Dendian").map(str::to_owned),
        dsp: attrs.get("Ddsp").map(str::to_owned),
        mve: attrs.get("Dmve").map(str::to_owned),
        trustzone: attrs.get("Dtz").map(str::to_owned),
        mpu: attrs.get("Dmpu").map(str::to_owned),
        clock_hz: attrs.get("Dclock").and_then(parse_u64),
        rust_target: target,
        source_pack_vendor: pack.vendor.clone(),
        source_pack_name: pack.name.clone(),
        source_pack_version: pack.version.clone(),
        source_url: pack.url.clone(),
        source_pdsc: pack.pdsc_name.clone(),
    }
}

fn processor_attrs(node: Node<'_, '_>) -> ProcessorAttrs {
    let mut values = BTreeMap::new();
    for attr in node.attributes() {
        if attr.name() == "Pname" {
            continue;
        }
        values.insert(attr.name().to_owned(), attr.value().to_owned());
    }
    ProcessorAttrs { values }
}

fn normalize_vendor(value: &str) -> String {
    value.split(':').next().unwrap_or(value).trim().to_owned()
}

fn normalize_fpu(value: &str) -> String {
    match value.trim() {
        "0" => "NO_FPU".to_owned(),
        "1" => "FPU".to_owned(),
        other => other.to_owned(),
    }
}

fn parse_u64(value: &str) -> Option<u64> {
    let value = value.trim();
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()
    } else {
        value.parse().ok()
    }
}

fn direct_child<'a, 'input: 'a>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}

fn direct_child_text(node: Node<'_, '_>, name: &str) -> Option<String> {
    direct_child(node, name)
        .and_then(|n| n.text())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inherited_gd32_style_processor() {
        let path = Path::new("tests/fixtures/inheritance.pdsc");
        let records = parse_pdsc(path).unwrap();
        let gd32 = records.iter().find(|r| r.device == "GD32F303CG").unwrap();

        assert_eq!(gd32.vendor, "GigaDevice");
        assert_eq!(gd32.core.as_deref(), Some("Cortex-M4"));
        assert_eq!(gd32.fpu.as_deref(), Some("SP_FPU"));
        assert_eq!(gd32.endian.as_deref(), Some("Little-endian"));
        assert_eq!(gd32.rust_target.as_deref(), Some("thumbv7em-none-eabihf"));
    }

    #[test]
    fn parses_variants() {
        let path = Path::new("tests/fixtures/inheritance.pdsc");
        let records = parse_pdsc(path).unwrap();
        let variant = records.iter().find(|r| r.device == "GD32F303CGT6").unwrap();

        assert_eq!(variant.device_kind, "variant");
        assert_eq!(variant.parent_device.as_deref(), Some("GD32F303CG"));
        assert_eq!(
            variant.rust_target.as_deref(),
            Some("thumbv7em-none-eabihf")
        );
    }

    #[test]
    fn parses_named_processors_with_common_overrides() {
        let records = parse_pdsc(Path::new("tests/fixtures/multiprocessor.pdsc")).unwrap();
        let device: Vec<_> = records
            .iter()
            .filter(|record| record.device == "TEST123")
            .collect();

        assert_eq!(device.len(), 2);
        assert_eq!(device[0].processor.as_deref(), Some("CPU0"));
        assert_eq!(device[1].processor.as_deref(), Some("CPU1"));
        assert!(
            device
                .iter()
                .all(|record| record.endian.as_deref() == Some("Little-endian"))
        );
        assert!(
            device
                .iter()
                .all(|record| record.mpu.as_deref() == Some("MPU"))
        );

        assert_eq!(device[0].core.as_deref(), Some("Cortex-M4"));
        assert_eq!(device[0].fpu.as_deref(), Some("SP_FPU"));
        assert_eq!(device[0].clock_hz, Some(120_000_000));
        assert_eq!(device[0].dsp.as_deref(), Some("DSP"));

        assert_eq!(device[1].core.as_deref(), Some("Cortex-M33"));
        assert_eq!(device[1].fpu.as_deref(), Some("NO_FPU"));
        assert_eq!(device[1].clock_hz, Some(48_000_000));
        assert_eq!(device[1].dsp, None);

        let variants: Vec<_> = records
            .iter()
            .filter(|record| record.device == "TEST123A")
            .collect();
        assert_eq!(variants.len(), 2);
        assert!(variants.iter().all(|record| {
            record.device_kind == "variant" && record.parent_device.as_deref() == Some("TEST123")
        }));
        assert_eq!(variants[0].processor.as_deref(), Some("CPU0"));
        assert_eq!(variants[0].core.as_deref(), Some("Cortex-M4"));
        assert_eq!(variants[0].fpu.as_deref(), Some("SP_FPU"));
        assert_eq!(variants[0].endian.as_deref(), Some("Little-endian"));
        assert_eq!(variants[0].mpu.as_deref(), Some("MPU"));
        assert_eq!(variants[0].clock_hz, Some(120_000_000));
        assert_eq!(variants[0].dsp.as_deref(), Some("DSP"));
        assert_eq!(variants[1].processor.as_deref(), Some("CPU1"));
        assert_eq!(variants[1].core.as_deref(), Some("Cortex-M33"));
        assert_eq!(variants[1].fpu.as_deref(), Some("NO_FPU"));
        assert_eq!(variants[1].endian.as_deref(), Some("Little-endian"));
        assert_eq!(variants[1].mpu.as_deref(), Some("MPU"));
        assert_eq!(variants[1].clock_hz, Some(48_000_000));
        assert_eq!(variants[1].dsp, None);
    }

    #[test]
    fn normalizes_fpu_and_parses_integer_values() {
        assert_eq!(normalize_fpu("0"), "NO_FPU");
        assert_eq!(normalize_fpu(" 1 "), "FPU");
        assert_eq!(normalize_fpu("SP_FPU"), "SP_FPU");

        assert_eq!(parse_u64("120000000"), Some(120_000_000));
        assert_eq!(parse_u64("0x02DC6C00"), Some(48_000_000));
        assert_eq!(parse_u64("not-a-number"), None);
    }
}
