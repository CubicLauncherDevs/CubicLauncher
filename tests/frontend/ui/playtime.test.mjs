import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import {
	formatPlaytime,
	hasPlaytime,
} from "../../../src/lib/utils/playtime.ts";

function flatten(dict) {
	const flat = {};
	const walk = (node, prefix) => {
		for (const [key, value] of Object.entries(node)) {
			if (typeof value === "string") flat[prefix + key] = value;
			else walk(value, `${prefix}${key}.`);
		}
	};
	walk(dict, "");
	return flat;
}

function read(locale) {
	return flatten(
		JSON.parse(
			readFileSync(
				new URL(
					`../../../src/lib/i18n/${locale}.json`,
					import.meta.url,
				),
				"utf8",
			),
		),
	);
}

const en = read("en-US");
const es = read("es-ES");

let lookups = 0;

// Mismo coste que el `t` real: una búsqueda + un reemplazo de placeholders.
const t = (key, params) => {
	lookups += 1;
	const template = en[key];
	if (template === undefined) throw new Error(`missing translation: ${key}`);
	if (!params) return template;
	return template.replace(/\{(\w+)\}/g, (_, name) =>
		String(params[name] ?? `{${name}}`),
	);
};

test("playtime shows seconds below a minute", () => {
	expect(formatPlaytime(0, t)).toBe("0s");
	expect(formatPlaytime(45, t)).toBe("45s");
	expect(formatPlaytime(59, t)).toBe("59s");
});

test("playtime shows minutes below an hour", () => {
	expect(formatPlaytime(60, t)).toBe("1m");
	expect(formatPlaytime(3599, t)).toBe("59m");
});

test("playtime shows days, hours and minutes like Prism", () => {
	expect(formatPlaytime(3600, t)).toBe("1h");
	expect(formatPlaytime(3660, t)).toBe("1h 1m");
	expect(formatPlaytime(86400, t)).toBe("1d");
	expect(formatPlaytime(275193, t)).toBe("3d 4h 26m");
});

test("playtime omits components that are zero", () => {
	expect(formatPlaytime(90000, t)).toBe("1d 1h");
	expect(formatPlaytime(90060, t)).toBe("1d 1h 1m");
	expect(formatPlaytime(86460, t)).toBe("1d 1m");
	expect(formatPlaytime(172800, t)).toBe("2d");
});

test("playtime stays exact above 32-bit totals", () => {
	// 2^31 - 1 segundos.
	expect(formatPlaytime(2147483647, t)).toBe("24855d 3h 14m");
	// 115 años: el truncado de horas y minutos no desborda.
	expect(formatPlaytime(3650000000, t)).toBe("42245d 8h 53m");
});

test("playtime clamps invalid values to zero", () => {
	expect(formatPlaytime(-10, t)).toBe("0s");
	expect(formatPlaytime(Number.NaN, t)).toBe("0s");
	expect(formatPlaytime(Number.POSITIVE_INFINITY, t)).toBe("0s");
});

test("playtime row stays hidden while there is nothing played", () => {
	// Las instancias sin estrenar no deben enseñar un "0 s".
	expect(hasPlaytime(0)).toBe(false);
	expect(hasPlaytime(0.5)).toBe(false);
	expect(hasPlaytime(-10)).toBe(false);
	expect(hasPlaytime(Number.NaN)).toBe(false);
	expect(hasPlaytime(Number.POSITIVE_INFINITY)).toBe(false);
	expect(hasPlaytime(1)).toBe(true);
	expect(hasPlaytime(275193)).toBe(true);
});

test("playtime resolves the whole label with a single translation lookup", () => {
	for (const seconds of [0, 90, 3600, 3660, 275193]) {
		lookups = 0;
		formatPlaytime(seconds, t);
		expect(lookups).toBe(1);
	}
});

test("playtime has a pattern per shape in both locales", () => {
	const keys = [
		"instanceView.playtimeSeconds",
		"instanceView.playtimeMinutes",
		"instanceView.playtimeHours",
		"instanceView.playtimeHoursMinutes",
		"instanceView.playtimeDays",
		"instanceView.playtimeDaysMinutes",
		"instanceView.playtimeDaysHours",
		"instanceView.playtimeDaysHoursMinutes",
	];
	for (const key of keys) {
		const placeholders = en[key].match(/\{\w+\}/g);
		expect(placeholders).not.toBeNull();
		expect(es[key].match(/\{\w+\}/g)).toEqual(placeholders);
	}
});

test("playtime stays cheap in bulk", () => {
	lookups = 0;
	const started = performance.now();
	for (let i = 0; i < 100_000; i += 1) {
		formatPlaytime(i * 137, t);
	}
	const elapsed = performance.now() - started;
	expect(lookups).toBe(100_000);
	expect(elapsed).toBeLessThan(2000);
});
