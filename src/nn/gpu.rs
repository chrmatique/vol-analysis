use std::process::Command;

use crate::data::models::GpuAdapterInfo;

/// GPU information collected via nvidia-smi or rocm-smi/amd-smi
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub vram_total_mb: u64,
    pub vram_used_mb: u64,
    pub utilization_percent: f32,
    pub temperature_c: f32,
}

/// Detect all WGPU-capable adapters (NVIDIA, AMD, Intel) via wgpu.
/// Returns an empty vec if wgpu fails to enumerate (e.g. no GPU drivers).
pub fn detect_wgpu_adapters() -> Vec<GpuAdapterInfo> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let mut adapters = Vec::new();
    for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
        let info = adapter.get_info();
        adapters.push(GpuAdapterInfo {
            name: info.name.clone(),
        });
    }
    adapters
}

/// Detect an NVIDIA GPU by querying nvidia-smi.
/// Returns `Some(GpuInfo)` if an NVIDIA GPU is found, `None` otherwise.
pub fn detect_nvidia_gpu() -> Option<GpuInfo> {
    query_nvidia_smi()
}

/// Detect an AMD GPU by querying rocm-smi (Linux) or amd-smi (Windows).
/// Returns `Some(GpuInfo)` if AMD stats are available, `None` otherwise.
pub fn detect_amd_gpu() -> Option<GpuInfo> {
    #[cfg(target_os = "linux")]
    return query_rocm_smi();

    #[cfg(windows)]
    return query_amd_smi();

    #[cfg(not(any(target_os = "linux", windows)))]
    return None;
}

/// Poll live GPU stats (VRAM usage, utilization, temperature).
/// Prefers NVIDIA (nvidia-smi), then AMD (rocm-smi/amd-smi).
pub fn poll_gpu_stats() -> Option<GpuInfo> {
    detect_nvidia_gpu().or_else(detect_amd_gpu)
}

/// Validate that the WGPU GPU backend is usable by running a small tensor computation.
///
/// Performs a 4×4 matrix multiply on the WGPU device to verify allocation, compute,
/// and readback. Returns the adapter name on success or an error description on failure.
/// Call this before starting GPU training to gate on a known-good backend.
pub fn validate_gpu() -> Result<String, String> {
    use burn::backend::Wgpu;
    use burn::tensor::Tensor;
    type B = Wgpu;

    let device = <B as burn::tensor::backend::Backend>::Device::default();

    // 4×4 matmul: ones × ones = all 4.0 -- tests allocation, compute, and readback
    let a = Tensor::<B, 2>::ones([4, 4], &device);
    let b = Tensor::<B, 2>::ones([4, 4], &device);
    let c = a.matmul(b);
    let vals = c
        .into_data()
        .to_vec::<f32>()
        .map_err(|e| format!("GPU tensor readback failed: {e:?}"))?;

    if vals.len() != 16 || vals.iter().any(|&v| (v - 4.0).abs() > 0.01) {
        return Err("GPU computation produced incorrect results".into());
    }

    let name = detect_wgpu_adapters()
        .into_iter()
        .next()
        .map(|a| a.name)
        .unwrap_or_else(|| "Unknown GPU".into());

    Ok(name)
}

fn query_nvidia_smi() -> Option<GpuInfo> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.used,utilization.gpu,temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next()?.trim().to_string();
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

    if parts.len() < 5 {
        return None;
    }

    Some(GpuInfo {
        name: parts[0].to_string(),
        vram_total_mb: parts[1].parse().unwrap_or(0),
        vram_used_mb: parts[2].parse().unwrap_or(0),
        utilization_percent: parts[3].parse().unwrap_or(0.0),
        temperature_c: parts[4].parse().unwrap_or(0.0),
    })
}

#[cfg(target_os = "linux")]
fn query_rocm_smi() -> Option<GpuInfo> {
    let output = Command::new("rocm-smi")
        .args(["--showmeminfo", "vram", "--showuse", "--showtemp"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let name = "AMD GPU".to_string();
    let mut vram_total_mb = 0u64;
    let mut vram_used_mb = 0u64;
    let mut utilization_percent = 0.0f32;
    let mut temperature_c = 0.0f32;

    for line in stdout.lines() {
        let line = line.trim();
        if line.contains("GPU use") {
            if let Some(pct) = line.split('%').next().and_then(|s| s.split_whitespace().last()) {
                utilization_percent = pct.parse().unwrap_or(0.0);
            }
        } else if line.contains("Temperature") {
            if let Some(temp) = line.split("Temperature (Sensor").next().and_then(|s| {
                s.split_whitespace()
                    .find(|w| w.ends_with('C'))
                    .and_then(|w| w.trim_end_matches('C').parse::<f32>().ok())
            }) {
                temperature_c = temp;
            } else if let Some(t) = line.split(' ').find_map(|w| w.parse::<f32>().ok()) {
                temperature_c = t;
            }
        } else if line.contains("VRAM Total Memory") || line.contains("vram") {
            let mb = line
                .split_whitespace()
                .find_map(|w| w.parse::<u64>().ok())
                .unwrap_or(0);
            if vram_total_mb == 0 {
                vram_total_mb = mb;
            } else {
                vram_used_mb = mb;
            }
        }
    }

    if vram_total_mb == 0 && vram_used_mb == 0 {
        return None;
    }

    Some(GpuInfo {
        name,
        vram_total_mb: if vram_total_mb > 0 {
            vram_total_mb
        } else {
            vram_used_mb * 2
        },
        vram_used_mb,
        utilization_percent,
        temperature_c,
    })
}

#[cfg(target_os = "linux")]
fn query_amd_smi() -> Option<GpuInfo> {
    None::<GpuInfo>
}

#[cfg(windows)]
fn query_rocm_smi() -> Option<GpuInfo> {
    None::<GpuInfo>
}

#[cfg(windows)]
fn query_amd_smi() -> Option<GpuInfo> {
    let output = Command::new("amd-smi")
        .args(["metric"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let name = "AMD GPU".to_string();
    let mut vram_total_mb = 0u64;
    let mut vram_used_mb = 0u64;
    let mut utilization_percent = 0.0f32;
    let mut temperature_c = 0.0f32;

    for line in stdout.lines() {
        let line = line.trim().to_lowercase();
        if line.contains("memory") {
            if let Some(mb) = line
                .split_whitespace()
                .find_map(|w| w.replace(",", "").parse::<u64>().ok())
            {
                if vram_total_mb == 0 {
                    vram_total_mb = mb;
                } else {
                    vram_used_mb = mb;
                }
            }
        } else if line.contains("utilization") || line.contains("gpu use") {
            if let Some(pct) = line
                .split_whitespace()
                .find_map(|w| w.trim_end_matches('%').parse::<f32>().ok())
            {
                utilization_percent = pct;
            }
        } else if line.contains("temperature") || line.contains("temp") {
            if let Some(t) = line
                .split_whitespace()
                .find_map(|w| w.trim_end_matches('c').parse::<f32>().ok())
            {
                temperature_c = t;
            }
        }
    }

    if vram_total_mb == 0 && utilization_percent == 0.0 && temperature_c == 0.0 {
        return None;
    }

    Some(GpuInfo {
        name,
        vram_total_mb: if vram_total_mb > 0 {
            vram_total_mb
        } else {
            8192
        },
        vram_used_mb,
        utilization_percent,
        temperature_c,
    })
}

#[cfg(not(any(target_os = "linux", windows)))]
fn query_rocm_smi() -> Option<GpuInfo> {
    None::<GpuInfo>
}

#[cfg(not(any(target_os = "linux", windows)))]
fn query_amd_smi() -> Option<GpuInfo> {
    None::<GpuInfo>
}

#[cfg(test)]
mod tests {
    use super::*;

    /// validate_gpu() must return Ok or Err without panicking -- even on CI machines
    /// without a real GPU (WGPU will fall back to its software/null adapter).
    #[test]
    fn validate_gpu_does_not_panic() {
        let result = validate_gpu();
        // We only assert the absence of a panic; Ok vs Err depends on the host GPU.
        match result {
            Ok(name) => {
                assert!(!name.is_empty(), "GPU name should be non-empty on success");
            }
            Err(reason) => {
                assert!(!reason.is_empty(), "Error message should be non-empty on failure");
            }
        }
    }
}
