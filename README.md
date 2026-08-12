# cmsis-rust-target-db

从公开的 Keil/Open-CMSIS-Pack 索引可复现生成 MCU 元数据与 Rust 裸机目标数据库。

本仓库不手工维护 `MCU -> Rust target` 静态表。自动任务定期下载 CMSIS-Pack PDSC 元数据，解析 CMSIS 设备层级继承，并根据最终生效的处理器属性推导 Rust 目标。

## 数据示例

每个生成记录的结构如下：

```json
{
  "vendor": "GigaDevice",
  "device": "GD32F303CG",
  "device_kind": "device",
  "parent_device": null,
  "processor": null,
  "processor_units": null,
  "core": "Cortex-M4",
  "core_version": null,
  "fpu": "FPU",
  "endian": "Little-endian",
  "dsp": null,
  "mve": null,
  "trustzone": null,
  "mpu": "1",
  "clock_hz": 120000000,
  "rust_target": "thumbv7em-none-eabihf",
  "source_pack_vendor": "GigaDevice",
  "source_pack_name": "GD32F30x_DFP",
  "source_pack_version": "2.2.1",
  "source_url": "https://gd32mcu.com/data/documents/pack/",
  "source_pdsc": "GigaDevice.GD32F30x_DFP.pdsc"
}
```

更新工作流使用 [Keil 设备页中的 `GD32F303CG`](https://www.keil.arm.com/devices/gigadevice-gd32f303cg/processors/) 作为端到端检查。当前上游 PDSC 使用已废弃的 `Dfpu="1"` 表示已实现 FPU，生成器将其无损规范化为 `FPU`；该设备的 `Cortex-M4 + FPU + Little-endian` 属性解析为 `thumbv7em-none-eabihf`。

## 生成文件

- `data/devices.jsonl`：规范的逐行 JSON 数据集。
- `data/devices.csv`：便于电子表格或 SQL 导入的扁平表格。
- `data/metadata.json`：记录数、PDSC 文件数、模式版本及源 Pack Index 的 SHA-256。

记录按确定顺序输出，行粒度为 `(device, processor, source pack)`。多处理器 MCU 的每个处理器各占一行；variant 单独输出，并通过 `parent_device` 指向父设备。同一逻辑设备可能由多个活跃 pack 提供，因此会输出多行，且处理器属性可能互相冲突。`query` 返回全部匹配记录，不定义来源优先级，也不自动合并；消费者应使用 `source_pack_vendor`、`source_pack_name`、`source_pack_version` 和 `source_pdsc` 选择所需来源。

## 数据流

```text
Keil public index.pidx
        ↓
cpackget init -C 1 --all-pdsc-files
        ↓
*.pdsc
        ↓
family → subFamily → device → variant 属性继承
        ↓
最终生效的处理器属性
        ↓
Core / FPU / Endian / DSP / MVE / TrustZone / MPU
        ↓
Rust target resolver
        ↓
devices.jsonl + devices.csv + metadata.json
```

## Rust 目标映射

生成器仅推导 Rust 内置裸机 Thumb 目标覆盖的 Cortex-M 内核。未知内核或非 Cortex-M 内核的 `rust_target` 保持 `null`，不会猜测。

| CMSIS 内核 | FPU | Rust 目标 |
|---|---|---|
| SC000 / Cortex-M0 / M0+ / M1 | 无 | `thumbv6m-none-eabi` |
| SC300 / Cortex-M3 | 无 | `thumbv7m-none-eabi` |
| Cortex-M4 / M7 | 无 | `thumbv7em-none-eabi` |
| Cortex-M4 / M7 | 硬件 FPU | `thumbv7em-none-eabihf` |
| Cortex-M23 / ARMV8MBL | 无 | `thumbv8m.base-none-eabi` |
| Cortex-M33 / M35P / M52 / M55 / M85 / ARMV8MML | 无 | `thumbv8m.main-none-eabi` |
| Cortex-M33 / M35P / M52 / M55 / M85 / ARMV8MML | 硬件 FPU | `thumbv8m.main-none-eabihf` |

## 本地完整生成

先从 [CMSIS-Toolbox 官方发布页](https://github.com/Open-CMSIS-Pack/cmsis-toolbox/releases/tag/2.14.1) 安装固定版本 `2.14.1`，并确保 `cpackget` 可从 `PATH` 调用，然后执行：

```bash
export CMSIS_PACK_ROOT="$PWD/.cmsis-packs"

cpackget -V
cpackget init -C 1 https://www.keil.com/pack/index.pidx --all-pdsc-files

cargo run --release -- generate \
  --pdsc-root "$CMSIS_PACK_ROOT/.Web" \
  --index-file "$CMSIS_PACK_ROOT/.Web/index.pidx" \
  --out-dir data

cargo test --all-features
cargo run -- query GD32F303CG
```

`cpackget 2.2.1` 的 `--all-pdsc-files` 模式会排除已标记为 deprecated 的公开 pack。因此本仓库覆盖的是当前活跃的 Keil/Open-CMSIS-Pack 公开设备数据，不是包含所有历史废弃 MCU 的完整档案。对应行为见 [`cpackget` 官方议题 #246](https://github.com/Open-CMSIS-Pack/cpackget/issues/246)。

Keil 当前索引存在完全重复的条目。`cpackget 2.2.1` 默认并发下载时可能对同名 PDSC 产生 rename/delete 竞态，因此本地命令和自动更新都显式使用 `-C 1`。单并发全量同步约需 27 分钟，工作流为下载、生成、编译和测试预留 60 分钟。

`core`、`core_version`、`fpu`、`endian`、`dsp`、`mve`、`trustzone`、`mpu` 和 `clock_hz` 均依据上游 PDSC 继承后的处理器属性生成，未与芯片手册交叉验证。其中 `clock_hz` 忠实保留 `Dclock`，因此可能为 `0` 或明显异常值。Keil 当前索引还有仅大小写不同的 `NXP.Multicore.pdsc` 与 `NXP.MULTICORE.pdsc`；大小写不敏感的 macOS 文件系统只能保留后一份，所以本地 PDSC 计数为 1477，Linux 预计为 1478。冲突的旧包不含设备定义，不影响设备记录。

## 自动更新

`.github/workflows/update-data.yml` 每周三 `03:17 UTC` 运行，也可从 Actions 页面手工触发。它会：

1. 从 Open-CMSIS-Pack 官方 GitHub release 下载固定的 CMSIS-Toolbox `2.14.1`；
2. 使用 `cpackget init -C 1 --all-pdsc-files` 拉取公开索引及当前活跃 pack 的 PDSC；
3. 重新生成确定性数据文件；
4. 运行 Rust 测试并检查 `GD32F303CG` 的目标映射；
5. 仅在 `data/` 或 `Cargo.lock` 变化时提交到默认分支。

更新 workflow 声明完成自动提交所需的最小权限范围 `contents: write`。检出步骤不会持久保存凭据，显式 `GITHUB_TOKEN` 环境变量只在最终提交步骤中使用，并在推送后清除临时 Git 认证配置。仓库或组织的 Actions 策略必须允许该写权限，默认分支保护规则也必须允许此工作流直接推送；否则应调整策略或改用受保护分支允许的合并流程。参见 [GitHub Actions 工作流权限](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#permissions)。

公开仓库连续 60 天没有活动时，GitHub 可能自动停用 scheduled workflow；届时可在 Actions 页面重新启用。参见 [GitHub 文档](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/disable-and-enable-workflows)。

## CMSIS-Pack 属性继承

CMSIS 处理器属性不一定直接位于 `<device>` 元素上，也可能分布在 `family`、`subFamily`、`device` 和 `variant` 层级。生成器按层级应用属性，较低层级覆盖继承值。

对于多处理器设备，生成器分别跟踪带 `Pname` 的 `<processor>`；没有 `Pname` 的 `<processor>` 在该层级应用于全部处理器。

## 数据来源与许可证

本仓库发布规范化的事实元数据和溯源字段，不提交下载的 `.pack` 归档或完整上游 PDSC 语料。详情见 [DATA_PROVENANCE.md](DATA_PROVENANCE.md) 与 [REFERENCES.md](REFERENCES.md)。

根目录 MIT 许可证仅覆盖仓库自有的源代码和自编内容，不改变也不授予任何上游 pack、PDSC 或厂商内容的权利。本仓库不另行声明生成数据的许可证；使用者应自行核对并遵守适用的上游条款。
