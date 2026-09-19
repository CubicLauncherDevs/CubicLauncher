import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { launcherStore } from "$lib/state/state.svelte";
import { updaterState } from "$lib/state/updaterState.svelte";
import { createUpdateController } from "./updateController";

export const updater = createUpdateController(updaterState, {
	check: () => check({ timeout: 30_000 }),
	relaunch,
});

// Automatic updates now notify through the shared modal. Downloading and
// restarting are explicit actions, so startup never interrupts ongoing work.
export async function autoUpdate() {
	if (launcherStore.settings.auto_updates) {
		await updater.checkForUpdates(true);
	}
}
