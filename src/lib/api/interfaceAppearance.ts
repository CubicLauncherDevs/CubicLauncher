import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { interfacePreferences } from "$lib/utils/interfacePreferences";

export function applyInterfaceDensity(density: string): void {
	const value = interfacePreferences({ density }).density;
	const root = document.documentElement;
	const attribute = value === "theme" ? null : value;
	if (root.getAttribute("data-interface-density") === attribute) return;
	if (attribute === null) root.removeAttribute("data-interface-density");
	else root.setAttribute("data-interface-density", attribute);
}

const settled = Promise.resolve();
let zoomQueue = settled;
let appliedScale: number | undefined;
let lastRequest: { scale: number; promise: Promise<void> } | undefined;

/** Native page zoom keeps viewport units, portals, pointer coordinates and
 * ResizeObserver in the same CSS coordinate system. Never rewrite theme CSS. */
export function applyInterfaceScale(scale: number): Promise<void> {
	const normalized = interfacePreferences({ scale }).scale;
	if (!isTauri()) return settled;
	if (lastRequest?.scale === normalized) return lastRequest.promise;
	if (!lastRequest && appliedScale === normalized) return settled;
	const task = zoomQueue.then(async () => {
		if (appliedScale === normalized) return;
		await getCurrentWebview().setZoom(normalized / 100);
		appliedScale = normalized;
	});
	// A rejected native request must not block subsequent changes or a reset.
	zoomQueue = task.catch(() => {});
	const request = { scale: normalized, promise: task };
	lastRequest = request;
	const finish = () => {
		if (lastRequest === request) lastRequest = undefined;
	};
	void task.then(finish, finish);
	return task;
}
