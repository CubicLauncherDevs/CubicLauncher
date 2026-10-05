import type { Update } from "@tauri-apps/plugin-updater";

export type UpdateStatus =
	| "idle"
	| "checking"
	| "available"
	| "downloading"
	| "ready"
	| "installing"
	| "restarting"
	| "updated"
	| "error";

type UpdateOperation = "check" | "download" | "install" | "restart";
type UpdateHandle = Pick<
	Update,
	"version" | "body" | "download" | "install" | "close"
>;

export interface UpdaterState {
	open: boolean;
	status: UpdateStatus;
	update: { version: string; body?: string } | null;
	error: string | null;
	failedOperation: UpdateOperation | null;
	downloadedBytes: number;
	totalBytes: number | null;
	progress: number | null;
	lastChecked: number | null;
	lastAttempt: number | null;
}

export function createUpdaterState(): UpdaterState {
	return {
		open: false,
		status: "idle",
		update: null,
		error: null,
		failedOperation: null,
		downloadedBytes: 0,
		totalBytes: null,
		progress: null,
		lastChecked: null,
		lastAttempt: null,
	};
}

export function createUpdateController(
	state: UpdaterState,
	dependencies: {
		check: () => Promise<UpdateHandle | null>;
		relaunch: () => Promise<void>;
		canAutoPrompt?: () => boolean;
		now?: () => number;
	},
) {
	let update: UpdateHandle | null = null;
	let dismissals = 0;
	const announcedVersions = new Set<string>();
	const now = () => dependencies.now?.() ?? Date.now();
	let checkInFlight: Promise<void> | null = null;
	let autoRequested = false;

	function fail(operation: UpdateOperation, error: unknown) {
		state.status = "error";
		state.failedOperation = operation;
		state.error = error instanceof Error ? error.message : String(error);
	}

	function begin(status: UpdateStatus) {
		state.status = status;
		state.error = null;
		state.failedOperation = null;
	}

	function announce() {
		if (
			!update ||
			announcedVersions.has(update.version) ||
			dependencies.canAutoPrompt?.() === false
		)
			return;
		announcedVersions.add(update.version);
		state.open = true;
	}

	async function checkForUpdates(automatic = false) {
		if (checkInFlight) {
			autoRequested ||= automatic;
			return checkInFlight;
		}
		// Never replace a handle that owns an in-flight or prepared download.
		if (
			state.status !== "idle" &&
			state.status !== "available" &&
			state.status !== "updated" &&
			!(state.status === "error" && state.failedOperation === "check")
		) {
			if (automatic && state.status === "ready") announce();
			return;
		}
		// Focus/online events can arrive in bursts. Explicit checks always bypass
		// this cooldown; failures can be retried sooner than successful checks.
		const cooldown = state.status === "error" ? 30_000 : 5 * 60_000;
		if (
			automatic &&
			state.lastAttempt !== null &&
			now() - state.lastAttempt < cooldown
		) {
			notifyAvailable();
			return;
		}
		const dismissedAtStart = dismissals;
		autoRequested = automatic;
		state.lastAttempt = now();
		begin("checking");
		checkInFlight = (async () => {
			try {
				const candidate = await dependencies.check();
				const previous = update;
				update = candidate;
				state.lastChecked = now();
				state.update = update
					? { version: update.version, body: update.body }
					: null;
				state.status = update ? "available" : "updated";
				if (update && dismissedAtStart !== dismissals)
					announcedVersions.add(update.version);
				if (autoRequested && dismissedAtStart === dismissals)
					announce();
				if (previous && previous !== candidate) {
					try {
						await previous.close();
					} catch {
						/* Already released natively. */
					}
				}
			} catch (error) {
				fail("check", error);
			}
		})().finally(() => {
			checkInFlight = null;
		});
		return checkInFlight;
	}

	function notifyAvailable() {
		if (state.status === "available" || state.status === "ready")
			announce();
	}

	function open() {
		state.open = true;
		// Reopening a failed operation exposes Retry rather than restarting it.
		if (state.status === "idle" || state.status === "updated") {
			void checkForUpdates();
		}
	}

	function close() {
		dismissals++;
		if (update) announcedVersions.add(update.version);
		state.open = false;
	}

	async function download() {
		if (
			!update ||
			(state.status !== "available" &&
				!(
					state.status === "error" &&
					(state.failedOperation === "download" ||
						state.failedOperation === "install")
				))
		)
			return;
		begin("downloading");
		state.downloadedBytes = 0;
		state.totalBytes = null;
		state.progress = null;
		try {
			await update.download((event) => {
				if (event.event === "Started") {
					state.totalBytes = event.data.contentLength || null;
					state.progress = state.totalBytes ? 0 : null;
				} else if (event.event === "Progress") {
					state.downloadedBytes += event.data.chunkLength;
					state.progress = state.totalBytes
						? Math.min(
								100,
								Math.round(
									(state.downloadedBytes / state.totalBytes) *
										100,
								),
							)
						: null;
				}
				// Finished only reports transfer completion. The promise also covers
				// signature validation and creation of the installable resource.
			});
			state.progress = 100;
			state.status = "ready";
		} catch (error) {
			fail("download", error);
		}
	}

	async function restart() {
		begin("restarting");
		try {
			await dependencies.relaunch();
		} catch (error) {
			fail("restart", error);
		}
	}

	async function install() {
		if (!update || state.status !== "ready") return;
		begin("installing");
		try {
			await update.install();
		} catch (error) {
			// The native installer may consume its bytes even on failure. Retry
			// downloads again instead of reusing a potentially consumed resource.
			fail("install", error);
			return;
		}
		// Windows exits during install; other platforms restart explicitly.
		try {
			await update.close();
		} catch {
			/* Already released by the installer. */
		}
		update = null;
		await restart();
	}

	async function retry() {
		if (state.status !== "error") return;
		switch (state.failedOperation) {
			case "check":
				await checkForUpdates();
				break;
			case "download":
			case "install":
				await download();
				break;
			case "restart":
				await restart();
				break;
		}
	}

	return {
		open,
		close,
		checkForUpdates,
		notifyAvailable,
		download,
		install,
		retry,
	};
}
