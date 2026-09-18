import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { interfacePreferences } from "$lib/utils/interfacePreferences";

export function applyInterfaceDensity(density: string): void {
	const value = interfacePreferences({ density }).density;
	const root = document.documentElement;
	if (value === "theme") root.removeAttribute("data-interface-density");
	else root.setAttribute("data-interface-density", value);
}

let zoomQueue = Promise.resolve();
let appliedScale: number | undefined;

/** Native page zoom keeps viewport units, portals, pointer coordinates and
 * ResizeObserver in the same CSS coordinate system. Never rewrite theme CSS. */
export function applyInterfaceScale(scale: number): Promise<void> {
	const normalized = interfacePreferences({ scale }).scale;
	const task = zoomQueue.then(async () => {
		if (!isTauri() || appliedScale === normalized) return;
		await getCurrentWebview().setZoom(normalized / 100);
		appliedScale = normalized;
	});
	// A rejected native request must not block subsequent changes or a reset.
	zoomQueue = task.catch(() => {});
	return task;
}
