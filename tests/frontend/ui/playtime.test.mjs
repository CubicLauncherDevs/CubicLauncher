import { expect, test } from "bun:test";
import { formatPlaytime } from "../../../src/lib/utils/playtime.ts";

const stub = (key, params) => {
	const entries = Object.entries(params ?? {})
		.map(([k, v]) => `${k}=${v}`)
		.join(",");
	return entries ? `${key}(${entries})` : key;
};

test("playtime shows seconds below a minute", () => {
	expect(formatPlaytime(0, stub)).toBe(
		"instanceView.playtimeSeconds(seconds=0)",
	);
	expect(formatPlaytime(45, stub)).toBe(
		"instanceView.playtimeSeconds(seconds=45)",
	);
});

test("playtime shows minutes and hours", () => {
	expect(formatPlaytime(60, stub)).toBe(
		"instanceView.playtimeMinutes(minutes=1)",
	);
	expect(formatPlaytime(3599, stub)).toBe(
		"instanceView.playtimeMinutes(minutes=59)",
	);
	expect(formatPlaytime(3600, stub)).toBe(
		"instanceView.playtimeHoursMinutes(hours=1,minutes=0)",
	);
	expect(formatPlaytime(3661, stub)).toBe(
		"instanceView.playtimeHoursMinutes(hours=1,minutes=1)",
	);
});

test("playtime clamps invalid values to zero", () => {
	expect(formatPlaytime(-10, stub)).toBe(
		"instanceView.playtimeSeconds(seconds=0)",
	);
	expect(formatPlaytime(Number.NaN, stub)).toBe(
		"instanceView.playtimeSeconds(seconds=0)",
	);
});
