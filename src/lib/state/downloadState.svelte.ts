import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { SvelteSet } from "svelte/reactivity";
import { getDownloadQueue } from "$lib/api/cubicApi";
import type { AppEvent } from "$lib/types/types";

const activeVersionDownloads = new SvelteSet<string>();

let _initialized = false;
let _unlisten: Promise<UnlistenFn> | null = null;

export async function initDownloadState(): Promise<void> {
	if (_initialized) return;
	_initialized = true;

	try {
		const queue = await getDownloadQueue();
		for (const item of queue) {
			if (item.status === "pending" || item.status === "downloading") {
				activeVersionDownloads.add(item.version);
			}
		}
	} catch (e) {
		console.error("Error initializing download state:", e);
	}

	_unlisten = listen<AppEvent>("app-event", (event) => {
		const payload = event.payload;
		if (payload.type === "DEnqueue") {
			activeVersionDownloads.add(payload.data.version);
		} else if (payload.type === "DFinish" || payload.type === "DError") {
			activeVersionDownloads.delete(payload.data.version);
		}
	});
}

export function isVersionDownloading(versionId: string): boolean {
	return activeVersionDownloads.has(versionId);
}

export function destroyDownloadState(): void {
	if (_unlisten) {
		_unlisten.then((u) => u()).catch(() => {});
		_unlisten = null;
	}
	_initialized = false;
	activeVersionDownloads.clear();
}
