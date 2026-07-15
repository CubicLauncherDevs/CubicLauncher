import { launcherStore } from "$lib/state/state.svelte";
import es from "./es.json";
import en from "./en.json";
import fr from "./fr.json";
import de from "./de.json";

type NestedKeys<T, Prefix extends string = ""> = {
	[K in keyof T & string]: T[K] extends string
		? `${Prefix}${K}`
		: T[K] extends object
			? NestedKeys<T[K], `${Prefix}${K}.`>
			: never;
}[keyof T & string];

export type TranslationKey = Exclude<NestedKeys<typeof es>, "id">;

type DictValue = string | { [key: string]: DictValue };
type LocaleDict = Record<string, DictValue>;

const dicts: Record<string, LocaleDict> = { es, en, fr, de };

const flatCache = new Map<string, Record<string, string>>();
let enFlat: Record<string, string> | null = null;

function flatten(
	obj: Record<string, DictValue>,
	prefix = "",
): Record<string, string> {
	const result: Record<string, string> = {};
	for (const key in obj) {
		if (key === "id") continue;
		const val = obj[key];
		if (typeof val === "string") {
			result[prefix + key] = val;
		} else {
			Object.assign(result, flatten(val, prefix + key + "."));
		}
	}
	return result;
}

function getFlat(lang: string): Record<string, string> {
	if (lang === "en" && enFlat) return enFlat;

	let cached = flatCache.get(lang);
	if (!cached) {
		const dict = dicts[lang];
		cached = dict && typeof dict === "object" ? flatten(dict) : {};
		flatCache.set(lang, cached);
	}
	if (lang === "en") enFlat = cached;
	return cached;
}

// Pre-cache English for fallback
getFlat("en");

export function t(
	key: TranslationKey,
	params?: Record<string, string | number>,
): string;
export function t(key: string, params?: Record<string, string | number>): string;
export function t(
	key: string,
	params?: Record<string, string | number>,
): string {
	const lang = launcherStore.settings?.language || "es";
	const flat = getFlat(lang);

	const result = flat[key];
	if (result !== undefined) {
		if (!params) return result;
		return result.replace(/\{(\w+)\}/g, (_, name) =>
			String(params[name] ?? `{${name}}`),
		);
	}

	// Fallback to English
	if (lang !== "en" && enFlat) {
		const enResult = enFlat[key];
		if (enResult !== undefined) {
			if (!params) return enResult;
			return enResult.replace(/\{(\w+)\}/g, (_, name) =>
				String(params[name] ?? `{${name}}`),
			);
		}
	}

	return key;
}

export const locales = [
	{ code: "es", label: "Español" },
	{ code: "en", label: "English" },
	{ code: "fr", label: "Français" },
	{ code: "de", label: "Deutsch" },
] as const;
