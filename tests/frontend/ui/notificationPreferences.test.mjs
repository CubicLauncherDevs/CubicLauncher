import { expect, test } from "bun:test";
import {
	DEFAULT_NOTIFICATION_PREFERENCES,
	notificationPreferences,
	notificationTimeout,
} from "../../../src/lib/components/ui/notificationPreferences.ts";

test("legacy fallbacks are stable and explicit preferences take precedence", () => {
	const legacy = notificationPreferences({ prominent_notifications: true });
	expect(legacy.position).toBe("top-center");
	expect(legacy.enabled).toBe(false);
	expect(DEFAULT_NOTIFICATION_PREFERENCES.enabled).toBe(false);
	expect(notificationPreferences({ prominent_notifications: true })).toBe(
		legacy,
	);
	expect(notificationPreferences({})).toBe(DEFAULT_NOTIFICATION_PREFERENCES);
	const custom = {
		...legacy,
		uppercase_title: false,
		position: "bottom-left",
	};
	expect(
		notificationPreferences({
			prominent_notifications: true,
			notification_preferences: custom,
		}),
	).toBe(custom);
	expect(legacy.uppercase_title).toBe(true);
});

test("custom durations apply to timed notices without expiring persistent ones", () => {
	for (const seconds of [3, 5, 8, 30]) {
		expect(notificationTimeout(8000, seconds, true)).toBe(seconds * 1000);
		expect(notificationTimeout(1000, seconds, true)).toBe(seconds * 1000);
		expect(notificationTimeout(0, seconds, true)).toBe(0);
		expect(notificationTimeout(undefined, seconds, true)).toBeUndefined();
		expect(notificationTimeout(-1, seconds, true)).toBe(-1);
	}
});

test("classic mode preserves each notification's original timeout", () => {
	for (const timeout of [undefined, 0, 4000, 5000, 8000]) {
		expect(notificationTimeout(timeout, 30)).toBe(timeout);
		expect(notificationTimeout(timeout, 3, false)).toBe(timeout);
	}
});
