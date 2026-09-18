export const NOTIFICATION_POSITIONS = [
	"top-left",
	"top-center",
	"top-right",
	"bottom-left",
	"bottom-center",
	"bottom-right",
] as const;
export const NOTIFICATION_SIZES = ["compact", "normal", "wide"] as const;

export interface NotificationPreferences {
	enabled: boolean;
	position: (typeof NOTIFICATION_POSITIONS)[number];
	size: (typeof NOTIFICATION_SIZES)[number];
	title_size: number;
	message_size: number;
	uppercase_title: boolean;
	bold_title: boolean;
	duration_seconds: number;
}

export const DEFAULT_NOTIFICATION_PREFERENCES: Readonly<NotificationPreferences> =
	Object.freeze({
		enabled: false,
		position: "top-right",
		size: "compact",
		title_size: 13,
		message_size: 11,
		uppercase_title: false,
		bold_title: false,
		duration_seconds: 5,
	});

const LEGACY_PROMINENT: Readonly<NotificationPreferences> = Object.freeze({
	enabled: false,
	position: "top-center",
	size: "wide",
	title_size: 18,
	message_size: 16,
	uppercase_title: true,
	bold_title: true,
	duration_seconds: 8,
});

// Stable fallbacks avoid allocating per toast before settings have synchronized.
export function notificationPreferences(settings: {
	notification_preferences?: NotificationPreferences | null;
	prominent_notifications?: boolean;
}): Readonly<NotificationPreferences> {
	return (
		settings.notification_preferences ??
		(settings.prominent_notifications
			? LEGACY_PROMINENT
			: DEFAULT_NOTIFICATION_PREFERENCES)
	);
}

export function notificationTimeout(
	timeout: number | undefined,
	durationSeconds: number,
	enabled = false,
): number | undefined {
	// Missing/zero timeouts belong to persistent notices, not timed toasts.
	return enabled && timeout && timeout > 0 ? durationSeconds * 1000 : timeout;
}
