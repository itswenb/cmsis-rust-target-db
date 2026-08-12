# 数据来源与溯源

生成的 MCU 记录来自以下公开 CMSIS-Pack 索引：

```text
https://www.keil.com/pack/index.pidx
```

更新工作流从 [Open-CMSIS-Pack 官方发布页](https://github.com/Open-CMSIS-Pack/cmsis-toolbox/releases/tag/2.14.1) 下载固定版本的 CMSIS-Toolbox `2.14.1`，使用其中的 `cpackget 2.2.1` 执行：

```bash
cpackget init -C 1 https://www.keil.com/pack/index.pidx --all-pdsc-files
```

Keil 当前索引存在完全重复的条目。`cpackget 2.2.1` 默认并发下载时可能对同名 PDSC 产生 rename/delete 竞态，因此命令显式使用 `-C 1` 单并发下载，避免静默得到不完整的 `.Web` 数据。

随后，仓库中的 Rust 生成器解析 `.Web` 目录下的 PDSC 元数据，计算 `family → subFamily → device → variant` 层级继承后的处理器属性，并生成 JSONL、CSV 与 metadata 文件。`metadata.json` 保存源 `index.pidx` 的 SHA-256；每条设备记录保留源 pack 的厂商、名称、版本、URL 和 PDSC 文件名，便于回溯。

## 覆盖范围

当前 `cpackget 2.2.1` 在 `--all-pdsc-files` 模式下排除标记为 deprecated 的公开 pack。因此数据集表示生成时 Keil/Open-CMSIS-Pack 公开索引中的活跃设备数据，不保证包含历史上发布过的全部废弃设备。

仓库不会提交下载的 `.pack` 归档或完整上游 PDSC 语料，只提交规范化生成结果及其溯源信息。

## 权利边界

CMSIS-Pack 是数据格式与分发机制，各厂商 pack 可能带有各自的许可证或使用条款。根目录 MIT 许可证仅覆盖本仓库自有的源代码和自编内容，不会重新许可上游 pack、PDSC 或厂商内容，也不会改变其权利归属。

本仓库不另行声明生成数据的许可证。使用或再分发生成结果前，应根据记录中的溯源字段核对并遵守适用的上游条款。
