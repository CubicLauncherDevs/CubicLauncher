import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { t } from "$lib/i18n";
import {
	showInfo,
	showSuccess,
	showError,
	showErrorParsed,
	addNotification,
	updateNotification,
	removeNotification,
} from "$lib/state/state.svelte";
import { launcherStore } from "$lib/state/state.svelte";

let cachedUpdate: Update | null = null;
let isUpdating = false;

type ProgressCallback = (progress: number) => void;

export async function checkForUpdates(silent = false): Promise<boolean> {
	if (isUpdating) return false;

	try {
		const update = await check();

		if (!update) {
			launcherStore.pendingUpdate = null;
			cachedUpdate = null;
			if (!silent)
				showInfo("Actualizaciones", "Ya tenés la última versión.");
			return false;
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
		return true;
	} catch (err) {
		if (!silent) showErrorParsed(err);
		return false;
	}
}

/**
 * Downloads the cached update and tracks progress.
 * Does NOT install — call installUpdate() for that.
 */
export async function downloadUpdate(
	onProgress?: ProgressCallback,
): Promise<boolean> {
	if (!cachedUpdate) {
		showError("Sin update", "No hay ninguna actualización disponible.");
		return false;
	}

	try {
		launcherStore.updateProgress = 0;
		launcherStore.updateDownloaded = false;
		isUpdating = true;

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
					onProgress?.(launcherStore.updateProgress);
					break;
				case "Finished":
					launcherStore.updateProgress = 100;
					launcherStore.updateDownloaded = true;
					onProgress?.(100);
					break;
			}
		});

		showSuccess(
			"Descarga completa",
			"La actualización está lista para instalar.",
		);
		return true;
	} catch (err) {
		showErrorParsed(err);
		launcherStore.updateProgress = 0;
		return false;
	} finally {
		isUpdating = false;
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
 * One-shot helper used by manual buttons.
 */
export async function downloadAndInstall() {
	const ok = await downloadUpdate();
	if (ok && launcherStore.updateDownloaded) {
		await installUpdate();
	}
}

/**
 * Fully automatic flow used at startup when `auto_updates` is enabled.
 * Checks, downloads, installs and relaunches without user interaction.
 */
export async function autoUpdate() {
	if (isUpdating) return;
	if (!launcherStore.settings.auto_updates) return;

	isUpdating = true;
	let notificationId: string | null = null;

	try {
		const hasUpdate = await checkForUpdates(true);
		if (!hasUpdate || !cachedUpdate) {
			isUpdating = false;
			return;
		}

		const version = cachedUpdate.version;

		notificationId = addNotification(
			t("updater.availableTitle", { version }),
			t("updater.availableMessage"),
			"info",
			0,
		);

		await downloadUpdate((progress) => {
			if (!notificationId) return;
			updateNotification(notificationId, {
				title: t("updater.downloadingTitle", { version }),
				message: t("updater.progress", { progress }),
				progress,
				timeout: 0,
			});
		});

		if (!launcherStore.updateDownloaded) {
			if (notificationId) removeNotification(notificationId);
			isUpdating = false;
			return;
		}

		if (notificationId) {
			updateNotification(notificationId, {
				title: t("updater.installingTitle", { version }),
				message: t("updater.installingMessage"),
				type: "info",
				progress: 100,
				timeout: 0,
			});
		}

		await installUpdate();
	} catch (err) {
		if (notificationId) {
			updateNotification(notificationId, {
				title: t("updater.errorTitle"),
				message: t("errors.GENERIC", { error: String(err) }),
				type: "error",
				timeout: 8000,
			});
		} else {
			showErrorParsed(err);
		}
	} finally {
		isUpdating = false;
	}
}
