import {
	launcherStore,
	showErrorParsed,
	showSuccess,
	clearPendingJreLaunch,
} from "../state/state.svelte";
import { listen } from "@tauri-apps/api/event";
import type {
	AppEvent,
	InstanceDto,
	InstOverrides,
	MinecraftUser,
} from "../types/types";
import {
	killInstance,
	getSettings,
	updateSettings,
	initDiscordPresence,
	shutdownDiscordPresence,
	launchInstance,
	getTags,
} from "./cubicApi";
import { applyTheme } from "./themeManager";
import { t } from "$lib/i18n";

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
			case "DFinish": {
				const pending = launcherStore.pendingJreLaunch;
				if (pending) {
					const jreVersionStr = `jre-${pending.version}`;
					if (payload.data.version === jreVersionStr) {
						handleJreInstalled(pending.version, pending.instance);
					}
				}
				break;
			}
			case "DError": {
				const pending = launcherStore.pendingJreLaunch;
				if (pending) {
					const jreVersionStr = `jre-${pending.version}`;
					if (payload.data.version === jreVersionStr) {
						clearPendingJreLaunch();
					}
				}
				break;
			}
			case "STChanged":
				clearTimeout(_settingsTimer);
				_settingsTimer = setTimeout(() => syncSettings(), 80);
				break;
			case "ThemeChanged":
				if (payload.data.id === launcherStore.settings.theme) {
					applyTheme(payload.data.id);
				}
				break;
			case "TagCreated":
				launcherStore.tags.push(payload.data.dto);
				break;
			case "TagUpdated": {
				const idx = launcherStore.tags.findIndex((t) => t.id === payload.data.dto.id);
				if (idx !== -1) launcherStore.tags[idx] = payload.data.dto;
				break;
			}
			case "TagDeleted":
				launcherStore.tags = launcherStore.tags.filter((t) => t.id !== payload.data.id);
				// also remove tag from instances
				for (const inst of launcherStore.loadedInstances) {
					const tagIdx = inst.tags.indexOf(payload.data.id);
					if (tagIdx !== -1) inst.tags.splice(tagIdx, 1);
				}
				break;
			case "InstanceTagsChanged": {
				const inst = launcherStore.loadedInstances.find((i) => i.uuid === payload.data.instance_uuid);
				if (inst) {
					inst.tags = payload.data.tags;
				}
				break;
			}
		}
	});
}

async function handleJreInstalled(version: number, instance: InstanceDto) {
	const versionStr = String(version);
	showSuccess(
		t("settings.java.installedVersion", { version: versionStr }),
		"",
	);

	if (version === 8 && !launcherStore.settings.jre8_managed) {
		launcherStore.settings.jre8_managed = true;
		await updateSettings(launcherStore.settings);
	} else if (version === 17 && !launcherStore.settings.jre17_managed) {
		launcherStore.settings.jre17_managed = true;
		await updateSettings(launcherStore.settings);
	} else if (version === 21 && !launcherStore.settings.jre21_managed) {
		launcherStore.settings.jre21_managed = true;
		await updateSettings(launcherStore.settings);
	} else if (version === 25 && !launcherStore.settings.jre25_managed) {
		launcherStore.settings.jre25_managed = true;
		await updateSettings(launcherStore.settings);
	}

	clearPendingJreLaunch();
	await launchInstance(instance);
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
	newOverrides?: InstOverrides | null,
): Promise<void> {
	try {
		await invoke("update_instance", {
			id: uuid,
			newName,
			newVersion,
			newIcon,
			newOverrides,
		});

		await getVersions();
	} catch (err) {
		showErrorParsed(err);
	}
}

export async function syncTags(): Promise<void> {
	const tags = await getTags();
	launcherStore.tags.splice(0, launcherStore.tags.length, ...tags);
}

export async function getVersions(): Promise<void> {
	const instances: InstanceDto[] = await invoke("get_instances");

	launcherStore.loadedInstances.splice(
		0,
		launcherStore.loadedInstances.length,
		...instances,
	);
}
