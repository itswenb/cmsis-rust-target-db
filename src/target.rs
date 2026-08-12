pub fn rust_target(core: Option<&str>, fpu: Option<&str>) -> Option<&'static str> {
    let core = core?.trim();
    let has_fpu = match fpu {
        Some(fpu) => has_hardware_fpu(fpu)?,
        None => matches!(core, "Cortex-M4F" | "Cortex-M7F"),
    };

    match core {
        "SC000" | "Cortex-M0" | "Cortex-M0+" | "Cortex-M1" => Some("thumbv6m-none-eabi"),
        "SC300" | "Cortex-M3" => Some("thumbv7m-none-eabi"),
        "Cortex-M4" | "Cortex-M4F" | "Cortex-M7" | "Cortex-M7F" => Some(if has_fpu {
            "thumbv7em-none-eabihf"
        } else {
            "thumbv7em-none-eabi"
        }),
        "Cortex-M23" | "ARMV8MBL" => Some("thumbv8m.base-none-eabi"),
        "Cortex-M33" | "Cortex-M35P" | "Cortex-M52" | "Cortex-M55" | "Cortex-M85" | "ARMV8MML" => {
            Some(if has_fpu {
                "thumbv8m.main-none-eabihf"
            } else {
                "thumbv8m.main-none-eabi"
            })
        }
        _ => None,
    }
}

fn has_hardware_fpu(fpu: &str) -> Option<bool> {
    match fpu.trim() {
        "" | "0" | "NO_FPU" => Some(false),
        "1" | "FPU" | "SP_FPU" | "DP_FPU" => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_cortex_m_targets() {
        assert_eq!(
            rust_target(Some("Cortex-M1"), Some("NO_FPU")),
            Some("thumbv6m-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M0+"), Some("NO_FPU")),
            Some("thumbv6m-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M3"), Some("NO_FPU")),
            Some("thumbv7m-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M4"), Some("NO_FPU")),
            Some("thumbv7em-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M4"), Some("SP_FPU")),
            Some("thumbv7em-none-eabihf")
        );
        assert_eq!(
            rust_target(Some("Cortex-M4F"), None),
            Some("thumbv7em-none-eabihf")
        );
        assert_eq!(
            rust_target(Some("Cortex-M4F"), Some("NO_FPU")),
            Some("thumbv7em-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M7"), Some("NO_FPU")),
            Some("thumbv7em-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M7"), Some("DP_FPU")),
            Some("thumbv7em-none-eabihf")
        );
        assert_eq!(
            rust_target(Some("Cortex-M23"), Some("NO_FPU")),
            Some("thumbv8m.base-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M33"), Some("SP_FPU")),
            Some("thumbv8m.main-none-eabihf")
        );
        assert_eq!(
            rust_target(Some("Cortex-M35P"), Some("NO_FPU")),
            Some("thumbv8m.main-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M35P"), Some("SP_FPU")),
            Some("thumbv8m.main-none-eabihf")
        );
        assert_eq!(
            rust_target(Some("Cortex-M55"), Some("NO_FPU")),
            Some("thumbv8m.main-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M55"), Some("SP_FPU")),
            Some("thumbv8m.main-none-eabihf")
        );
        assert_eq!(
            rust_target(Some("Cortex-M85"), Some("NO_FPU")),
            Some("thumbv8m.main-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M85"), Some("DP_FPU")),
            Some("thumbv8m.main-none-eabihf")
        );
        assert_eq!(
            rust_target(Some("SC000"), Some("NO_FPU")),
            Some("thumbv6m-none-eabi")
        );
        assert_eq!(
            rust_target(Some("SC300"), Some("NO_FPU")),
            Some("thumbv7m-none-eabi")
        );
        assert_eq!(
            rust_target(Some("ARMV8MBL"), Some("SP_FPU")),
            Some("thumbv8m.base-none-eabi")
        );
        assert_eq!(
            rust_target(Some("ARMV8MML"), Some("NO_FPU")),
            Some("thumbv8m.main-none-eabi")
        );
        assert_eq!(
            rust_target(Some("ARMV8MML"), Some("SP_FPU")),
            Some("thumbv8m.main-none-eabihf")
        );
        assert_eq!(
            rust_target(Some("Cortex-M52"), Some("NO_FPU")),
            Some("thumbv8m.main-none-eabi")
        );
        assert_eq!(
            rust_target(Some("Cortex-M52"), Some("SP_FPU")),
            Some("thumbv8m.main-none-eabihf")
        );
        assert_eq!(
            rust_target(Some("Cortex-M52"), Some("FPU")),
            Some("thumbv8m.main-none-eabihf")
        );
    }

    #[test]
    fn does_not_guess_unknown_cores() {
        assert_eq!(rust_target(Some("Cortex-A7"), None), None);
        assert_eq!(rust_target(Some("RISC-V"), None), None);
    }

    #[test]
    fn classifies_legacy_and_empty_fpu_values() {
        for (fpu, target) in [
            ("1", "thumbv7em-none-eabihf"),
            ("0", "thumbv7em-none-eabi"),
            (" ", "thumbv7em-none-eabi"),
        ] {
            assert_eq!(rust_target(Some("Cortex-M4"), Some(fpu)), Some(target));
        }
    }

    #[test]
    fn does_not_guess_unknown_fpu_types() {
        assert_eq!(rust_target(Some("Cortex-M4"), Some("UNKNOWN")), None);
        assert_eq!(rust_target(Some("Cortex-M4"), Some(" future_fpu ")), None);
    }
}
