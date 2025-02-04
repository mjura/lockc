use aya::{
    maps::{HashMap, MapError},
    Bpf,
};
use thiserror::Error;
use tracing::debug;

use crate::{
    bpfstructs::{
        accessed_path, container, container_id, container_policy_level, process, NewBpfstructError,
    },
    settings::SETTINGS,
};

#[derive(Error, Debug)]
pub enum MapOperationError {
    #[error(transparent)]
    Map(#[from] MapError),

    #[error(transparent)]
    NewBpfstruct(#[from] NewBpfstructError),
}

/// Registers the allowed directories for restricted and baseline containers in
/// BPF maps. Based on that information, mount_audit BPF prrogram will make a
/// decision whether to allow a bind mount for a given container.
pub fn init_allowed_paths(bpf: &mut Bpf) -> Result<(), MapOperationError> {
    let mut allowed_paths_mount_restricted: HashMap<_, u32, accessed_path> =
        bpf.map_mut("ap_mnt_restr")?.try_into()?;
    for (i, allowed_path_s) in SETTINGS.allowed_paths_mount_restricted.iter().enumerate() {
        let ap = accessed_path::new(allowed_path_s)?;
        allowed_paths_mount_restricted.insert(i as u32, ap, 0)?;
    }

    let mut allowed_paths_mount_baseline: HashMap<_, u32, accessed_path> =
        bpf.map_mut("ap_mnt_base")?.try_into()?;
    for (i, allowed_path_s) in SETTINGS.allowed_paths_mount_baseline.iter().enumerate() {
        let ap = accessed_path::new(allowed_path_s)?;
        allowed_paths_mount_baseline.insert(i as u32, ap, 0)?;
    }

    let mut allowed_paths_access_restricted: HashMap<_, u32, accessed_path> =
        bpf.map_mut("ap_acc_restr")?.try_into()?;
    for (i, allowed_path_s) in SETTINGS.allowed_paths_access_restricted.iter().enumerate() {
        let ap = accessed_path::new(allowed_path_s)?;
        allowed_paths_access_restricted.insert(i as u32, ap, 0)?;
    }

    let mut allowed_paths_access_baseline: HashMap<_, u32, accessed_path> =
        bpf.map_mut("ap_acc_base")?.try_into()?;
    for (i, allowed_path_s) in SETTINGS.allowed_paths_access_baseline.iter().enumerate() {
        let ap = accessed_path::new(allowed_path_s)?;
        allowed_paths_access_baseline.insert(i as u32, ap, 0)?;
    }

    let mut denied_paths_access_restricted: HashMap<_, u32, accessed_path> =
        bpf.map_mut("dp_acc_restr")?.try_into()?;
    for (i, allowed_path_s) in SETTINGS.denied_paths_access_restricted.iter().enumerate() {
        let ap = accessed_path::new(allowed_path_s)?;
        denied_paths_access_restricted.insert(i as u32, ap, 0)?;
    }

    let mut denied_paths_access_baseline: HashMap<_, u32, accessed_path> =
        bpf.map_mut("dp_acc_base")?.try_into()?;
    for (i, allowed_path_s) in SETTINGS.denied_paths_access_baseline.iter().enumerate() {
        let ap = accessed_path::new(allowed_path_s)?;
        denied_paths_access_baseline.insert(i as u32, ap, 0)?;
    }

    Ok(())
}

pub fn add_container(
    bpf: &mut Bpf,
    container_id: String,
    pid: i32,
    policy_level: container_policy_level,
) -> Result<(), MapOperationError> {
    debug!(
        container = container_id.as_str(),
        pid = pid,
        policy_level = policy_level,
        map = "containers",
        "adding container to eBPF map",
    );

    let mut containers: HashMap<_, container_id, container> =
        bpf.map_mut("containers")?.try_into()?;
    let container_key = container_id::new(&container_id)?;
    let container = container { policy_level };
    containers.insert(container_key, container, 0)?;

    let mut processes: HashMap<_, i32, process> = bpf.map_mut("processes")?.try_into()?;
    let process = process {
        container_id: container_key,
    };
    processes.insert(pid, process, 0)?;

    Ok(())
}

pub fn delete_container(bpf: &mut Bpf, container_id: String) -> Result<(), MapOperationError> {
    debug!(
        container = container_id.as_str(),
        map = "containers",
        "deleting container from eBPF map"
    );

    let mut containers: HashMap<_, container_id, container> =
        bpf.map_mut("containers")?.try_into()?;
    let container_key = container_id::new(&container_id)?;
    containers.remove(&container_key)?;

    let processes: HashMap<_, i32, process> = bpf.map("processes")?.try_into()?;
    let mut processes_mut: HashMap<_, i32, process> = bpf.map_mut("process")?.try_into()?;
    for res in processes.iter() {
        let (pid, process) = res?;
        if process.container_id.id == container_key.id {
            processes_mut.remove(&pid)?;
        }
    }

    Ok(())
}

pub fn add_process(bpf: &mut Bpf, container_id: String, pid: i32) -> Result<(), MapOperationError> {
    debug!(
        pid = pid,
        container = container_id.as_str(),
        map = "processes",
        "adding process to eBPF map",
    );

    let mut processes: HashMap<_, i32, process> = bpf.map_mut("processes")?.try_into()?;
    let container_key = container_id::new(&container_id)?;
    let process = process {
        container_id: container_key,
    };
    processes.insert(pid, process, 0)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::{Builder, TempDir};

    use crate::{bpfstructs::container_policy_level_POLICY_LEVEL_BASELINE, load::load_bpf};

    use super::*;

    fn tmp_path_base() -> TempDir {
        Builder::new()
            .prefix("lockc-temp")
            .rand_bytes(5)
            .tempdir_in("/sys/fs/bpf")
            .expect("Creating temporary dir in BPFFS failed")
    }

    #[test]
    fn test_init_allowed_paths() {
        let path_base = tmp_path_base();
        let mut bpf = load_bpf(path_base).expect("Loading BPF failed");
        init_allowed_paths(&mut bpf).expect("Initializing allowed paths failed");
    }

    #[test]
    fn test_add_container() {
        let path_base = tmp_path_base();
        let mut bpf = load_bpf(path_base).expect("Loading BPF failed");
        add_container(
            &mut bpf,
            "5833851e673d45fab4d12105bf61c3f4892b2bbf9c12d811db509a4f22475ec9".to_string(),
            42069,
            container_policy_level_POLICY_LEVEL_BASELINE,
        )
        .expect("Adding container failed");
    }
}
