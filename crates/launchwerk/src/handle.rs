// Copyright (C) 2025 Santiagolxx, CubicLauncher contributors
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::error::Error;
use crate::models::{Loader, VersionManifest};
use log::{debug, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::Mutex;
use tokio::sync::broadcast::{self, Receiver, Sender};
use uuid::Uuid;
use zellkern::resolvers::CommandBuilder;
use zellkern::{LaunchConfig, extract_natives};

// ─── Internal state ───────────────────────────────────────────────────────────

pub(crate) struct InstanceInner {
    pub uuid: Uuid,
    pub manifest: VersionManifest,
    pub loader: Loader,
    pub config: LaunchConfig,
    /// Root shared directory (contains libraries/, assets/, versions/).
    pub shared_dir: PathBuf,
    /// Per-instance game directory (saves, configs, mods).
    pub instance_dir: PathBuf,
    pub stdout_tx: Sender<String>,
    pub stderr_tx: Sender<String>,
    pub runtime: Mutex<InstanceRuntime>,
}

pub(crate) struct InstanceRuntime {
    pub process: Option<Child>,
}

// ─── Public handle ────────────────────────────────────────────────────────────

/// A handle to a prepared (or running) Minecraft instance.
///
/// Clone the receivers to watch stdout/stderr from multiple tasks.
pub struct InstanceHandle {
    pub(crate) inner: Arc<InstanceInner>,
    /// Subscribe to game stdout lines.
    pub stdout: Receiver<String>,
    /// Subscribe to game stderr lines.
    pub stderr: Receiver<String>,
}

impl InstanceHandle {
    /// UUID that uniquely identifies this instance within `Launchwerk`.
    pub fn id(&self) -> Uuid {
        self.inner.uuid
    }

    /// The parsed version manifest for this instance.
    pub fn manifest(&self) -> &VersionManifest {
        &self.inner.manifest
    }

    /// The detected / configured mod loader.
    pub fn loader(&self) -> &Loader {
        &self.inner.loader
    }

    /// Subscribe to a new stdout receiver (useful when sharing the handle
    /// between tasks – each caller gets an independent broadcast receiver).
    pub fn subscribe_stdout(&self) -> Receiver<String> {
        self.inner.stdout_tx.subscribe()
    }

    /// Subscribe to a new stderr receiver.
    pub fn subscribe_stderr(&self) -> Receiver<String> {
        self.inner.stderr_tx.subscribe()
    }

    /// Build the command, spawn the process and start pumping stdout/stderr
    /// into the broadcast channels.
    pub async fn launch(&self) -> Result<(), Error> {
        let inner = &self.inner;
        let config = &inner.config;

        // ── Extract natives ───────────────────────────────────────────────
        let lib_dir = inner.shared_dir.join("libraries");
        let natives_dir = inner
            .shared_dir
            .join("natives")
            .join(&inner.manifest.id_raw);

        extract_natives(&inner.manifest, &lib_dir, &natives_dir)?;

        // ── Build the command ─────────────────────────────────────────────
        let cmd_args = CommandBuilder::new(
            &inner.manifest,
            &inner.shared_dir,
            &inner.instance_dir,
            config,
        )
        .build()?;

        debug!(
            "Launching instance {} with {} args",
            inner.uuid,
            cmd_args.len()
        );
        for (i, arg) in cmd_args.iter().enumerate() {
            debug!("  [{i}] {arg}");
        }

        // ── Spawn ─────────────────────────────────────────────────────────
        let java = config.java_path.to_string_lossy().to_string();
        let mut cmd = tokio::process::Command::new(&java);
        cmd.args(&cmd_args[1..]) // cmd_args[0] is java itself
            .current_dir(&inner.instance_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // JAVA_HOME
        if let Some(java_home) = config.java_path.parent() {
            cmd.env("JAVA_HOME", java_home);
        }

        // ── Wayland → X11 fallback ────────────────────────────────────────
        // GLFW 3.4 auto-detects Wayland when WAYLAND_DISPLAY is set, but
        // Minecraft/Forge call glfwSetWindowPos() which Wayland doesn't
        // support. Force X11 by breaking the Wayland detection.
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            info!("Wayland detected, forcing X11 for GLFW compatibility");
            cmd.env("WAYLAND_DISPLAY", "")
                .env("XDG_SESSION_TYPE", "x11");
            if std::env::var("DISPLAY").is_err() {
                cmd.env("DISPLAY", ":0");
            }
        }

        for (k, v) in &config.env {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn()?;

        // ── Pump stdout ───────────────────────────────────────────────────
        let stdout_pipe = child.stdout.take().unwrap();
        let stdout_tx = inner.stdout_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout_pipe).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if stdout_tx.send(line).is_err() {
                    break; // no receivers left
                }
            }
        });

        // ── Pump stderr ───────────────────────────────────────────────────
        let stderr_pipe = child.stderr.take().unwrap();
        let stderr_tx = inner.stderr_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr_pipe).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if stderr_tx.send(line).is_err() {
                    break;
                }
            }
        });

        // ── Store child ───────────────────────────────────────────────────
        {
            let mut rt = self.inner.runtime.lock().await;
            rt.process = Some(child);
        }

        info!("Instance {} launched ({})", inner.uuid, inner.manifest.id);
        Ok(())
    }

    /// Wait for the running process to exit and return its exit code.
    /// Returns `None` if the instance was never launched.
    pub async fn wait(&self) -> Option<i32> {
        let mut rt = self.inner.runtime.lock().await;
        if let Some(child) = rt.process.as_mut() {
            match child.wait().await {
                Ok(status) => {
                    let code = status.code().unwrap_or(-1);
                    if status.success() {
                        info!("Instance {} exited cleanly", self.inner.uuid);
                    } else {
                        warn!("Instance {} exited with code {code}", self.inner.uuid);
                    }
                    Some(code)
                }
                Err(e) => {
                    warn!("Error waiting on instance {}: {e}", self.inner.uuid);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Send SIGKILL / TerminateProcess to the running instance.
    pub async fn kill(&self) -> Result<(), Error> {
        let mut rt = self.inner.runtime.lock().await;
        if let Some(child) = rt.process.as_mut() {
            child.kill().await?;
        }
        Ok(())
    }
}

// ─── Constructor (crate-internal) ────────────────────────────────────────────

impl InstanceHandle {
    pub(crate) fn new(
        manifest: VersionManifest,
        config: LaunchConfig,
        shared_dir: PathBuf,
        instance_dir: PathBuf,
    ) -> Self {
        let (stdout_tx, stdout_rx) = broadcast::channel(512);
        let (stderr_tx, stderr_rx) = broadcast::channel(512);
        let loader = Loader::from_version_id(&manifest.id_raw);
        let uuid = Uuid::new_v4();

        let inner = Arc::new(InstanceInner {
            uuid,
            manifest,
            loader,
            config,
            shared_dir,
            instance_dir,
            stdout_tx,
            stderr_tx,
            runtime: Mutex::new(InstanceRuntime { process: None }),
        });

        Self {
            inner,
            stdout: stdout_rx,
            stderr: stderr_rx,
        }
    }
}
