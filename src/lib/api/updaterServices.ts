import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { launcherStore } from "$lib/state/state.svelte";
import { updaterState } from "$lib/state/updaterState.svelte";
import { createUpdateController } from "./updateController";
import { createAutoUpdateScheduler } from "./autoUpdateScheduler";

let automaticPromptAllowed = false;

export const updater = createUpdateController(updaterState, {
	check: () => check({ timeout: 30_000 }),
	relaunch,
	canAutoPrompt: () =>
		automaticPromptAllowed &&
		launcherStore.settings.auto_updates &&
		!document.hidden,
});

// Automatic updates now notify through the shared modal. Downloading and
// restarting are explicit actions, so startup never interrupts ongoing work.
export async function autoUpdate() {
	if (launcherStore.settings.auto_updates) {
		await updater.checkForUpdates(true);
	}
}

export function startAutoUpdates() {
	automaticPromptAllowed = true;
	const scheduler = createAutoUpdateScheduler({
		check: autoUpdate,
		state: () => updaterState,
		canCheck: () => launcherStore.settings.auto_updates && !document.hidden,
		notifyAvailable: updater.notifyAvailable,
	});
	const focus = () => scheduler.wake();
	const online = () => scheduler.wake(true);
	window.addEventListener("focus", focus);
	window.addEventListener("online", online);
	document.addEventListener("visibilitychange", focus);
	return () => {
		automaticPromptAllowed = false;
		scheduler.stop();
		window.removeEventListener("focus", focus);
		window.removeEventListener("online", online);
		document.removeEventListener("visibilitychange", focus);
	};
}
