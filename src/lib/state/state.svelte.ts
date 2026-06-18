import type {
	InstanceDto,
	Settings,
	Notification,
	NotificationType,
} from "../types/types";
import { t } from "$lib/i18n";

export interface PendingUpdate {
	version: string;
	body?: string;
}

export interface LauncherState {
	loadedInstances: InstanceDto[];
	currentInstance: InstanceDto | null;
	runningInstances: string[];
	updateProgress: number;
	settings: Settings;
	notifications: Notification[];
	pendingUpdate: PendingUpdate | null;
	updateDownloaded: boolean;
}

export const launcherStore = $state<LauncherState>({
	loadedInstances: [],
	currentInstance: null,
	runningInstances: [],
	notifications: [],
	updateProgress: 0,
	pendingUpdate: null,
	updateDownloaded: false,
	settings: {
		user: [],
		active_user_idx: 0,
		min_memory: 1,
		max_memory: 2,
		jre8_path: "",
		jre8_managed: true,
		jre17_path: "",
		jre17_managed: true,
		jre21_path: "",
		jre21_managed: true,
		jre25_path: "",
		jre25_managed: true,
		language: "es",
		auto_updates: true,
		close_launcher_on_play: true,
		show_snapshots: false,
		show_alpha: false,
		jvm_args: "",
		env_vars: {},
		theme: "dark",
		discord_presence: true,
		show_tutorial: true,
	},
});

const MAX_NOTIFICATIONS = 5;

export function addNotification(
	title: string,
	message: string,
	type: NotificationType = "info",
	timeout = 5000,
) {
	const id = Math.random().toString(36).substring(2, 9);
	const notification: Notification = { id, title, message, type, timeout };

	launcherStore.notifications = [
		...launcherStore.notifications.slice(-(MAX_NOTIFICATIONS - 1)),
		notification,
	];

	return id;
}

export function removeNotification(id: string) {
	launcherStore.notifications = launcherStore.notifications.filter(
		(n) => n.id !== id,
	);
}

export function showError(title: string, message: string) {
	return addNotification(title, message, "error", 8000);
}

export function showSuccess(title: string, message: string) {
	return addNotification(title, message, "success", 4000);
}

export function showWarning(title: string, message: string) {
	return addNotification(title, message, "warning", 6000);
}

export function showInfo(title: string, message: string) {
	return addNotification(title, message, "info", 4000);
}

export function showErrorParsed(rawError: unknown) {
	try {
		const parsed = JSON.parse(rawError as string);
		if (parsed.code) {
			const params = parsed.params || {};
			const msg = t(`errors.${parsed.code}`, params);
			addNotification(t("errors.title"), msg, "error", 8000);
			return;
		}
	} catch {
		// JSON parse failed, fall through
	}
	addNotification(t("errors.title"), String(rawError), "error", 8000);
}
