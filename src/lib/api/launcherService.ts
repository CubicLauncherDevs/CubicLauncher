import { launcherStore, showError } from "../state/state.svelte";
import { listen } from "@tauri-apps/api/event";
import type { AppEvent, InstanceDto } from "../types/types";
import {
	killInstance,
	getSettings,
	updateSettings,
	initDiscordPresence,
} from "./cubicApi";
import { applyTheme } from "./themeManager";

import { invoke } from "@tauri-apps/api/core";

let _listenerInitialized = false;
let debounceTimer: ReturnType<typeof setTimeout>;

export function initEventListeners(): void {
	if (_listenerInitialized) return;
	_listenerInitialized = true;

	listen<AppEvent>("app-event", (event) => {
		const payload = event.payload;

		switch (payload.type) {
			case "InstanceCreated":
				launcherStore.loadedInstances = [
					...launcherStore.loadedInstances,
					payload.data.dto,
				];
				break;
			case "InstanceEdited":
				clearTimeout(debounceTimer);
				debounceTimer = setTimeout(() => getVersions(), 100);
				break;
			case "InstanceDeleted":
				launcherStore.loadedInstances =
					launcherStore.loadedInstances.filter(
						(i) => i.uuid !== payload.data.id,
					);
				break;
			case "DFinishRuntime":
				break;
			case "STChanged":
				syncSettings();
				break;
			case "ThemeChanged":
				console.log("ThemeChanged event:", payload.data.id);
				if (payload.data.id === launcherStore.settings.theme) {
					applyTheme(payload.data.id);
				}
				break;
		}
	});
}

export async function syncSettings(): Promise<void> {
	const settings = await getSettings();
	if (settings) {
		launcherStore.settings = settings;
	}
}

export async function saveSettings(): Promise<void> {
	const prev = launcherStore.settings.discord_presence;
	await updateSettings(launcherStore.settings);
	if (launcherStore.settings.discord_presence && !prev) {
		await initDiscordPresence();
	}
}

export async function killInst(uuid: string): Promise<void> {
	try {
		await killInstance(uuid, () => {
			launcherStore.runningInstances =
				launcherStore.runningInstances.filter((item) => item !== uuid);
		});
	} catch (err) {
		console.error("Error al matar instancia:", err);
	}
}

export async function deleteInst(uuid: string): Promise<void> {
	try {
		await invoke("delete_instance", { id: uuid });

		launcherStore.loadedInstances = launcherStore.loadedInstances.filter(
			(instance) => instance.uuid !== uuid,
		);
	} catch (err) {
		console.error("Error al eliminar instancia:", err);

		showError("Error", "No se pudo eliminar la instancia");
	}
}

export async function renameInst(uuid: string, newName: string): Promise<void> {
	try {
		await invoke("rename_instance", {
			id: uuid,
			newName,
		});

		await getVersions();
	} catch (err) {
		console.error("Error al renombrar instancia:", err);

		showError("Error", "No se pudo renombrar la instancia");
	}
}

export async function updateInst(
	uuid: string,
	newName?: string,
	newVersion?: string,
	newIcon?: string | null,
): Promise<void> {
	try {
		await invoke("update_instance", {
			id: uuid,
			newName,
			newVersion,
			newIcon,
		});

		await getVersions();
	} catch (err) {
		console.error("Error al actualizar instancia:", err);

		showError("Error", "No se pudo actualizar la instancia");
	}
}

export async function getVersions(): Promise<void> {
	const instances: InstanceDto[] = await invoke("get_instances");

	launcherStore.loadedInstances = instances;
}
