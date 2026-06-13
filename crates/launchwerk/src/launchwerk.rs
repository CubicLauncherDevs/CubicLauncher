// Copyright (C) 2025 Santiagolxx, CubicLauncher contributors
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::handle::{InstanceHandle, InstanceInner};
use crate::models::VersionManifest;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use zellkern::LaunchConfig;

/// Top-level manager for Minecraft instances.
///
/// # Example
/// ```no_run
/// # use launchwerk::{Launchwerk, LaunchConfig};
/// # use launchwerk::models::VersionManifest;
/// # use std::path::PathBuf;
/// # #[tokio::main] async fn main() {
/// let lw = Launchwerk::new(PathBuf::from("/home/user/.cubic/shared"));
/// let manifest = VersionManifest::from_file("versions/1.21/1.21.json").unwrap();
/// let config = LaunchConfig::default();
/// let handle = lw.prepare(manifest, config, PathBuf::from("/home/user/.cubic/instances/1.21"));
/// handle.launch().await.unwrap();
/// # }
/// ```
pub struct Launchwerk {
    ///   `<shared_dir>/libraries/`
    ///   `<shared_dir>/assets/`
    ///   `<shared_dir>/versions/<id>/<id>.jar`
    pub shared_dir: PathBuf,
    instances: DashMap<Uuid, Arc<InstanceInner>>,
}

impl Launchwerk {
    pub fn new(shared_dir: PathBuf) -> Self {
        Self {
            shared_dir,
            instances: DashMap::new(),
        }
    }

    pub fn prepare(
        &self,
        manifest: VersionManifest,
        config: LaunchConfig,
        instance_dir: PathBuf,
    ) -> InstanceHandle {
        let handle = InstanceHandle::new(manifest, config, self.shared_dir.clone(), instance_dir);

        self.instances
            .insert(handle.id(), Arc::clone(&handle.inner));

        handle
    }

    pub fn get(&self, id: Uuid) -> Option<InstanceHandle> {
        self.instances.get(&id).map(|inner| InstanceHandle {
            stdout: inner.stdout_tx.subscribe(),
            stderr: inner.stderr_tx.subscribe(),
            inner: Arc::clone(&*inner),
        })
    }

    pub fn remove(&self, id: Uuid) {
        self.instances.remove(&id);
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}
