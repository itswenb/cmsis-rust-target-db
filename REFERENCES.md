# 官方参考资料

生成器设计与自动化使用以下官方资料：

- CMSIS-Pack 设备层级与属性继承：
  - [设备说明](https://open-cmsis-pack.github.io/Open-CMSIS-Pack-Spec/main/html/pdsc_devices_pg.html)
  - [family 与 subFamily 说明](https://open-cmsis-pack.github.io/Open-CMSIS-Pack-Spec/main/html/pdsc_family_pg.html)
- CMSIS-Toolbox 与 `cpackget`：
  - [CMSIS-Toolbox 构建工具文档](https://open-cmsis-pack.github.io/cmsis-toolbox/build-tools/)
  - [CMSIS-Toolbox 2.14.1 官方发布页](https://github.com/Open-CMSIS-Pack/cmsis-toolbox/releases/tag/2.14.1)
  - [`cpackget` 官方使用说明](https://github.com/Open-CMSIS-Pack/cpackget/blob/main/README.md)
  - [deprecated pack 排除行为](https://github.com/Open-CMSIS-Pack/cpackget/issues/246)
- 已知设备检查：
  - [Keil：GigaDevice GD32F303CG 处理器属性](https://www.keil.arm.com/devices/gigadevice-gd32f303cg/processors/)
- Rust 内置 Cortex-M 目标：
  - [`thumbv6m-none-eabi`](https://doc.rust-lang.org/rustc/platform-support/thumbv6m-none-eabi.html)
  - [`thumbv7m-none-eabi`](https://doc.rust-lang.org/rustc/platform-support/thumbv7m-none-eabi.html)
  - [`thumbv7em-none-eabi`](https://doc.rust-lang.org/rustc/platform-support/thumbv7em-none-eabi.html)
  - [`thumbv8m.base-none-eabi`](https://doc.rust-lang.org/rustc/platform-support/thumbv8m.base-none-eabi.html)
  - [`thumbv8m.main-none-eabi`](https://doc.rust-lang.org/rustc/platform-support/thumbv8m.main-none-eabi.html)
- GitHub Actions：
  - [`actions/checkout` 官方仓库](https://github.com/actions/checkout)
  - [`GITHUB_TOKEN` 与 `permissions`](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#permissions)
  - [停用与重新启用工作流](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/disable-and-enable-workflows)
