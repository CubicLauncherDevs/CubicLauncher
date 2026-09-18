import { launcherStore } from "$lib/state/state.svelte";
import type es from "./es-ES.json";
import en from "./en-US.json";
import { i18nLoader, locales, type LocaleEntry } from "./loader.svelte";
import { invoke } from "@tauri-apps/api/core";

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
type StoredLocale = {
	code: string;
	id: string;
	data: string;
};

type CachedLocale = {
	flat: Record<string, string>;
	version: string | null;
	bytes: number;
};
const MAX_CACHED_LOCALES = 3;
const MAX_CACHE_BYTES = 4 * 1024 * 1024;
const flatCache = new Map<string, CachedLocale>();
const pendingFetches = new Map<string, Promise<void>>();
const failedLocales = new Set<string>();
const scheduled = new Set<string>();
const EMPTY: Record<string, string> = {};
const enFlat = flatten(en as LocaleDict);
let retainedBytes = 0;
let lastUsedLanguage = "";
let catalogPromise: Promise<void> | null = null;
let catalogCheckedAt = 0;

export function isBundled(lang: string): boolean {
	return lang === "es" || lang === "en";
}

export function isFetched(lang: string): boolean {
	return i18nLoader.fetched.has(lang);
}

function localeFlag(id: string): string {
	const region = id
		.split("-")
		.slice(1)
		.find((part) => /^[a-z]{2}$/i.test(part));
	if (!region) return "";

	return String.fromCodePoint(
		...region
			.toUpperCase()
			.split("")
			.map((char) => char.charCodeAt(0) + 127397),
	);
}

function localeEntryFromDict(
	code: string,
	id: string,
	dict: LocaleDict,
): LocaleEntry {
	const languages = dict.languages;
	const ownLabel =
		typeof languages === "object" && typeof languages[code] === "string"
			? languages[code]
			: id;

	return { code, id, label: ownLabel, flag: localeFlag(id) };
}

function addStoredLocale(entry: LocaleEntry): void {
	if (!locales.some((locale) => locale.id === entry.id)) {
		locales.push(entry);
	}
}

function activateLocale(code: string, id: string, dict: LocaleDict): void {
	const flat = flatten(dict);
	const bytes = Object.entries(flat).reduce(
		(total, [key, value]) => total + (key.length + value.length) * 2,
		0,
	);
	removeCachedLocale(code);
	flatCache.set(code, {
		flat,
		version: typeof dict.version === "string" ? dict.version : null,
		bytes,
	});
	retainedBytes += bytes;
	pruneCache();
	i18nLoader.fetched.add(code);
	i18nLoader.dictVersion[code] = (i18nLoader.dictVersion[code] ?? 0) + 1;
	failedLocales.delete(code);
	addStoredLocale(localeEntryFromDict(code, id, dict));
}

function removeCachedLocale(code: string): void {
	const entry = flatCache.get(code);
	if (entry) retainedBytes -= entry.bytes;
	flatCache.delete(code);
}

function pruneCache(): void {
	const active = launcherStore.settings?.language || "es";
	for (const code of flatCache.keys()) {
		if (
			flatCache.size <= MAX_CACHED_LOCALES &&
			retainedBytes <= MAX_CACHE_BYTES
		)
			break;
		// The active translation remains usable even if a custom dictionary is large.
		if (code !== active) removeCachedLocale(code);
	}
}

function markFailed(code: string): void {
	if (failedLocales.size >= 32)
		failedLocales.delete(failedLocales.values().next().value!);
	failedLocales.add(code);
}

function flatten(
	obj: Record<string, DictValue>,
	prefix = "",
): Record<string, string> {
	const result: Record<string, string> = {};
	for (const key in obj) {
		if (key === "id" || (!prefix && key === "version")) continue;
		const val = obj[key];
		if (typeof val === "string") {
			result[prefix + key] = val;
		} else if (val && typeof val === "object") {
			Object.assign(result, flatten(val, prefix + key + "."));
		}
	}
	return result;
}

function getFlat(lang: string): Record<string, string> {
	if (lang === "en") {
		if (lastUsedLanguage !== lang) {
			lastUsedLanguage = lang;
			pruneCache();
		}
		return enFlat;
	}
	void (i18nLoader.dictVersion[lang] ?? 0);
	const cached = flatCache.get(lang);
	if (cached) {
		if (lastUsedLanguage !== lang) {
			lastUsedLanguage = lang;
			flatCache.delete(lang);
			flatCache.set(lang, cached);
			pruneCache();
		}
		return cached.flat;
	}
	if (
		!pendingFetches.has(lang) &&
		!failedLocales.has(lang) &&
		!scheduled.has(lang)
	) {
		scheduled.add(lang);
		// Translation lookups may run inside a Svelte derived expression.
		queueMicrotask(() => {
			scheduled.delete(lang);
			if ((launcherStore.settings?.language || "es") === lang)
				void downloadLocale(lang);
		});
	}
	return EMPTY;
}

export async function downloadLocale(lang: string): Promise<void> {
	if (lang === "en") return;
	const pending = pendingFetches.get(lang);
	if (pending) return pending;

	const promise = Promise.resolve()
		.then(async () => {
			i18nLoader.loading = lang;
			let available = flatCache.has(lang);
			try {
				if (lang === "es") {
					if (!flatCache.has(lang)) {
						const { default: dict } = await import("./es-ES.json");
						activateLocale(lang, dict.id, dict as LocaleDict);
					}
					return;
				}
				if (!flatCache.has(lang)) {
					try {
						const stored = await invoke<StoredLocale | null>(
							"load_locale",
							{ code: lang },
						);
						if (stored) {
							activateStoredLocale(lang, stored);
							available = true;
						}
					} catch (error) {
						console.warn(
							`[i18n] Could not read local ${lang}:`,
							error,
						);
					}
				}
				// Native checks are shared by all windows and retain the offline copy.
				const updated = await invoke<StoredLocale | null>(
					"refresh_locale",
					{
						code: lang,
						knownVersion: flatCache.get(lang)?.version ?? null,
					},
				);
				if (updated) {
					activateStoredLocale(lang, updated);
					available = true;
				}
				if (!available) markFailed(lang);
			} catch (error) {
				if (!available) markFailed(lang);
				console.error(
					`[i18n] Failed to fetch locale "${lang}":`,
					error,
				);
			}
		})
		.finally(() => {
			pendingFetches.delete(lang);
			if (i18nLoader.loading === lang) {
				i18nLoader.loading = null;
			}
		});

	pendingFetches.set(lang, promise);
	return promise;
}

function activateStoredLocale(lang: string, stored: StoredLocale): void {
	const data = JSON.parse(stored.data) as LocaleDict;
	if (stored.code !== lang || data.id !== stored.id)
		throw new Error("Invalid locale response");
	activateLocale(lang, stored.id, data);
}

export function t(
	key: TranslationKey,
	params?: Record<string, string | number>,
): string;
export function t(
	key: string,
	params?: Record<string, string | number>,
): string;
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

export function loadAvailableLocales(): Promise<void> {
	if (catalogPromise) return catalogPromise;
	if (Date.now() - catalogCheckedAt < 30_000) return Promise.resolve();
	catalogPromise = invoke<(LocaleEntry & { installed: boolean })[]>(
		"list_locales",
	)
		.then((entries) => {
			const known = new Set(entries.map((entry) => entry.code));
			const extra = locales.filter((entry) => !known.has(entry.code));
			locales.splice(0, locales.length, ...entries, ...extra);
			for (const entry of entries)
				if (entry.installed) i18nLoader.fetched.add(entry.code);
			catalogCheckedAt = Date.now();
		})
		.catch((error) =>
			console.error("[i18n] Failed to list languages:", error),
		)
		.finally(() => {
			catalogPromise = null;
		});
	return catalogPromise;
}

export { locales };
