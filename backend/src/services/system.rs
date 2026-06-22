use serde::Serialize;
use std::sync::{Arc, Mutex};
use sysinfo::{Components, Disks, System};

#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub cpu_cores: usize,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_percent: f32,
    pub disks: Vec<DiskInfo>,
    pub temperatures: Vec<TempInfo>,
    pub hostname: String,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub percent: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TempInfo {
    pub label: String,
    pub celsius: f32,
}

pub struct SystemService {
    system: Arc<Mutex<System>>,
}

impl SystemService {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self {
            system: Arc::new(Mutex::new(system)),
        }
    }

    pub async fn collect(&self) -> SystemMetrics {
        let system = self.system.clone();
        tokio::task::spawn_blocking(move || collect_metrics(&system))
            .await
            .unwrap_or_else(|e| {
                tracing::error!("System metrics task failed: {e}");
                empty_metrics()
            })
    }
}

fn collect_metrics(system: &Arc<Mutex<System>>) -> SystemMetrics {
    let mut system = system.lock().unwrap();
    system.refresh_cpu_usage();
    system.refresh_memory();

    let cpu_usage = system.global_cpu_usage();
    let cpu_cores = system.cpus().len();
    let memory_total = system.total_memory();
    let memory_used = system.used_memory();
    let memory_percent = if memory_total > 0 {
        (memory_used as f32 / memory_total as f32) * 100.0
    } else {
        0.0
    };

    let disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            let used = total.saturating_sub(available);
            let percent = if total > 0 {
                (used as f32 / total as f32) * 100.0
            } else {
                0.0
            };
            DiskInfo {
                name: d.name().to_string_lossy().into(),
                mount_point: d.mount_point().to_string_lossy().into(),
                total,
                used,
                available,
                percent,
            }
        })
        .collect();

    let components = Components::new_with_refreshed_list();
    let temperatures = components
        .iter()
        .filter_map(|c| {
            let temp = c.temperature()?;
            if temp > 0.0 {
                Some(TempInfo {
                    label: c.label().to_string(),
                    celsius: temp,
                })
            } else {
                None
            }
        })
        .collect();

    let hostname = System::host_name().unwrap_or_else(|| "unknown".into());
    let uptime_secs = System::uptime();

    SystemMetrics {
        cpu_usage,
        cpu_cores,
        memory_total,
        memory_used,
        memory_percent,
        disks,
        temperatures,
        hostname,
        uptime_secs,
    }
}

fn empty_metrics() -> SystemMetrics {
    SystemMetrics {
        cpu_usage: 0.0,
        cpu_cores: 0,
        memory_total: 0,
        memory_used: 0,
        memory_percent: 0.0,
        disks: vec![],
        temperatures: vec![],
        hostname: "unknown".into(),
        uptime_secs: 0,
    }
}
