import type { UpdaterState } from "./updateController";

const CHECK_INTERVAL = 6 * 60 * 60_000;
const RETRY_DELAYS = [60_000, 5 * 60_000, 15 * 60_000, 60 * 60_000];
const ACTIVE_OPERATIONS = new Set([
	"checking",
	"downloading",
	"ready",
	"installing",
	"restarting",
]);
type TimerHandle = ReturnType<typeof setTimeout> | number;

/** One cancellable timer per main window. No networking or global DOM state here. */
export function createAutoUpdateScheduler(dependencies: {
	check: () => Promise<void>;
	state: () => Pick<UpdaterState, "status" | "failedOperation">;
	canCheck: () => boolean;
	notifyAvailable: () => void;
	now?: () => number;
	setTimer?: (callback: () => void, delay: number) => TimerHandle;
	clearTimer?: (timer: TimerHandle) => void;
}) {
	const now = dependencies.now ?? Date.now;
	const setTimer = dependencies.setTimer ?? setTimeout;
	const clearTimer = dependencies.clearTimer ?? clearTimeout;
	let timer: TimerHandle | undefined;
	let stopped = false;
	let checking = false;
	let failures = 0;
	let lastAttempt: number | null = null;
	let due = now() + 2_000;

	function schedule(delay: number) {
		if (timer !== undefined) clearTimer(timer);
		if (stopped) return;
		timer = setTimer(() => {
			timer = undefined;
			void run();
		}, delay);
	}

	async function run() {
		if (stopped || checking) return;
		if (!dependencies.canCheck()) {
			schedule(60_000);
			return;
		}
		const state = dependencies.state();
		if (
			ACTIVE_OPERATIONS.has(state.status) ||
			(state.status === "error" && state.failedOperation !== "check")
		) {
			dependencies.notifyAvailable();
			schedule(60_000);
			return;
		}
		if (now() < due) {
			schedule(due - now());
			return;
		}
		checking = true;
		lastAttempt = now();
		let failed = false;
		try {
			await dependencies.check();
			const result = dependencies.state();
			failed =
				result.status === "error" && result.failedOperation === "check";
		} catch {
			failed = true;
		} finally {
			checking = false;
			if (!stopped) {
				const delay = failed
					? RETRY_DELAYS[
							Math.min(failures++, RETRY_DELAYS.length - 1)
						]
					: CHECK_INTERVAL;
				if (!failed) failures = 0;
				due = now() + delay;
				schedule(delay);
			}
		}
	}

	schedule(2_000);
	return {
		wake(reconnected = false) {
			if (stopped || !dependencies.canCheck()) return;
			dependencies.notifyAvailable();
			const state = dependencies.state();
			const cooldown =
				reconnected &&
				state.status === "error" &&
				state.failedOperation === "check"
					? 30_000
					: 5 * 60_000;
			if (lastAttempt === null || now() - lastAttempt >= cooldown) {
				due = now();
				schedule(0);
			}
		},
		stop() {
			stopped = true;
			if (timer !== undefined) clearTimer(timer);
			timer = undefined;
		},
	};
}
