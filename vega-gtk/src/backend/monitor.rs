use lyra_vega_dbus::{MonitorClient, ProcessInfo, SystemMetrics};
use serde::Serialize;

use super::{CmdResult, State, dto, err};

dto!(Metrics from SystemMetrics {
    cpu_percent: f64, mem_used: u64, mem_total: u64, swap_used: u64, swap_total: u64,
    disk_read_bytes: u64, disk_write_bytes: u64, net_rx_bytes: u64, net_tx_bytes: u64,
    cpu_per_core: Vec<f64>, gpu_percent: f64, gpu_per_device: Vec<f64>,
});

#[derive(Serialize)]
pub struct Process {
    pid: u32,
    ppid: u32,
    name: String,
    user: String,
    cpu_percent: f64,
    memory: u64,
    state: String,
}

impl From<ProcessInfo> for Process {
    fn from(p: ProcessInfo) -> Self {
        Self {
            pid: p.pid,
            ppid: p.ppid,
            name: p.name,
            user: p.user,
            cpu_percent: p.cpu_percent.get(),
            memory: p.memory,
            state: p.state,
        }
    }
}

#[tauri::command]
pub async fn monitor_metrics(state: State<'_>) -> CmdResult<Metrics> {
    state
        .dbus()
        .await?
        .monitor()
        .metrics()
        .await
        .map(Into::into)
        .map_err(err)
}

#[tauri::command]
pub async fn monitor_processes(state: State<'_>) -> CmdResult<Vec<Process>> {
    let processes = state
        .dbus()
        .await?
        .monitor()
        .list_processes()
        .await
        .map_err(err)?;
    Ok(processes.into_iter().map(Process::from).collect())
}

#[tauri::command]
pub async fn monitor_kill(state: State<'_>, pid: u32) -> CmdResult<()> {
    state
        .dbus()
        .await?
        .monitor()
        .kill_process(pid)
        .await
        .map_err(err)
}
