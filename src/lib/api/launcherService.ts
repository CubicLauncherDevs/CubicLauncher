import { launcherStore, showErrorParsed } from "../state/state.svelte";
import { listen } from "@tauri-apps/api/event";
import type { AppEvent, InstanceDto, MinecraftUser } from "../types/types";
import {
	killInstance,
	getSettings,
	updateSettings,
	initDiscordPresence,
	shutdownDiscordPresence,
} from "./cubicApi";
import { applyTheme } from "./themeManager";

import { invoke } from "@tauri-apps/api/core";

export function getActiveUser(): MinecraftUser | null {
	const users = launcherStore.settings.user;
	const idx = launcherStore.settings.active_user_idx;
	if (users.length > 0 && idx >= 0 && idx < users.length) {
		return users[idx];
	}
	return null;
}

let _listenerInitialized = false;
let _instanceTimer: ReturnType<typeof setTimeout>;
let _settingsTimer: ReturnType<typeof setTimeout>;
let _localSettingsChange = false;

export function markLocalSettingsChange(): void {
	_localSettingsChange = true;
}

export function initEventListeners(): void {
	if (_listenerInitialized) return;
	_listenerInitialized = true;

	listen<AppEvent>("app-event", (event) => {
		const payload = event.payload;

		switch (payload.type) {
			case "InstanceCreated":
				launcherStore.loadedInstances.push(payload.data.dto);
				break;
			case "InstanceEdited":
				clearTimeout(_instanceTimer);
				_instanceTimer = setTimeout(() => getVersions(), 100);
				break;
			case "InstanceDeleted":
				{
					const idx = launcherStore.loadedInstances.findIndex(
						(i) => i.uuid === payload.data.id,
					);
					if (idx !== -1)
						launcherStore.loadedInstances.splice(idx, 1);
				}
				break;
			case "STChanged":
				clearTimeout(_settingsTimer);
				_settingsTimer = setTimeout(() => syncSettings(), 80);
				break;
			case "ThemeChanged":
				if (payload.data.id === launcherStore.settings.theme) {
					applyTheme(payload.data.id);
				}
				break;
		}
	});
}

export async function syncSettings(): Promise<void> {
	if (_localSettingsChange) {
		_localSettingsChange = false;
		return;
	}
	const settings = await getSettings();
	if (settings) {
		Object.assign(launcherStore.settings, settings);
	}
}

export async function saveSettings(): Promise<void> {
	_localSettingsChange = true;
	const prev = launcherStore.settings.discord_presence;
	await updateSettings(launcherStore.settings);
	if (launcherStore.settings.discord_presence && !prev) {
		await initDiscordPresence();
	} else if (!launcherStore.settings.discord_presence && prev) {
		await shutdownDiscordPresence();
	}
}

export async function killInst(uuid: string): Promise<void> {
	try {
		await killInstance(uuid, () => {
			const idx = launcherStore.runningInstances.indexOf(uuid);
			if (idx !== -1) launcherStore.runningInstances.splice(idx, 1);
		});
	} catch (err) {
		showErrorParsed(err);
	}
}

export async function deleteInst(uuid: string): Promise<void> {
	try {
		await invoke("delete_instance", { id: uuid });

		const idx = launcherStore.loadedInstances.findIndex(
			(instance) => instance.uuid === uuid,
		);
		if (idx !== -1) launcherStore.loadedInstances.splice(idx, 1);
	} catch (err) {
		showErrorParsed(err);
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
		showErrorParsed(err);
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
		showErrorParsed(err);
	}
}

export async function getVersions(): Promise<void> {
	const instances: InstanceDto[] = await invoke("get_instances");

	launcherStore.loadedInstances.splice(
		0,
		launcherStore.loadedInstances.length,
		...instances,
	);
}
