import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import {
	showInfo,
	showSuccess,
	showError,
	showErrorParsed,
} from "$lib/state/state.svelte";
import { launcherStore } from "$lib/state/state.svelte";

let cachedUpdate: Update | null = null;

export async function checkForUpdates(silent = false) {
	try {
		const update = await check();

		if (!update) {
			launcherStore.pendingUpdate = null;
			cachedUpdate = null;
			if (!silent)
				showInfo("Actualizaciones", "Ya tenés la última versión.");
			return;
		}

		cachedUpdate = update;
		launcherStore.pendingUpdate = {
			version: update.version,
			body: update.body ?? undefined,
		};

		showInfo(
			`Update disponible: v${update.version}`,
			"Podés descargarlo desde Ajustes.",
		);
	} catch (err) {
		if (!silent) showErrorParsed(err);
	}
}

/**
 * Downloads the cached update and tracks progress.
 * Does NOT install — call installUpdate() for that.
 */
export async function downloadUpdate() {
	if (!cachedUpdate) {
		showError("Sin update", "No hay ninguna actualización disponible.");
		return;
	}

	try {
		launcherStore.updateProgress = 0;
		launcherStore.updateDownloaded = false;

		let downloaded = 0;
		let total = 0;

		await cachedUpdate.download((event) => {
			switch (event.event) {
				case "Started":
					total = event.data.contentLength ?? 0;
					break;
				case "Progress":
					downloaded += event.data.chunkLength;
					launcherStore.updateProgress = total
						? Math.round((downloaded / total) * 100)
						: 0;
					break;
				case "Finished":
					launcherStore.updateProgress = 100;
					launcherStore.updateDownloaded = true;
					break;
			}
		});

		showSuccess(
			"Descarga completa",
			"La actualización está lista para instalar.",
		);
	} catch (err) {
		showErrorParsed(err);
		launcherStore.updateProgress = 0;
	}
}

/**
 * Installs the already-downloaded update and relaunches.
 */
export async function installUpdate() {
	if (!cachedUpdate) {
		showError("Sin update", "No hay ninguna actualización descargada.");
		return;
	}

	try {
		await cachedUpdate.install();
		await relaunch();
	} catch (err) {
		showErrorParsed(err);
	}
}

/**
 * Downloads and immediately installs (original one-shot behavior,
 * kept for convenience but no longer called on startup).
 */
export async function downloadAndInstall() {
	await downloadUpdate();
	if (launcherStore.updateDownloaded) {
		await installUpdate();
	}
}
