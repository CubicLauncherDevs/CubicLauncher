import { launcherStore } from "$lib/state/state.svelte";
import es from "./es-ES.json";
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
type LocaleVersion = {
	version: string;
};
type StoredLocale = {
	code: string;
	id: string;
	data: string;
};

const API_BASE = "https://i18n.cubiclauncher.org";

const bundled: Record<string, LocaleDict> = { es, en };
const fetchedDicts = new Map<string, LocaleDict>();
const flatCache = new Map<string, Record<string, string>>();
const pendingFetches = new Map<string, Promise<void>>();
const failedLocales = new Set<string>();

let enFlat: Record<string, string> | null = null;

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
	fetchedDicts.set(code, dict);
	flatCache.set(code, flatten(dict));
	i18nLoader.fetched.add(code);
	i18nLoader.dictVersion[code] = (i18nLoader.dictVersion[code] ?? 0) + 1;
	failedLocales.delete(code);
	addStoredLocale(localeEntryFromDict(code, id, dict));
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
		} else {
			Object.assign(result, flatten(val, prefix + key + "."));
		}
	}
	return result;
}

function getFlat(lang: string): Record<string, string> {
	if (lang === "en" && enFlat) return enFlat;
	if (!isBundled(lang)) {
		void (i18nLoader.dictVersion[lang] ?? 0);
	}

	let cached = flatCache.get(lang);
	if (!cached) {
		if (isBundled(lang)) {
			const dict = bundled[lang];
			cached = dict && typeof dict === "object" ? flatten(dict) : {};
		} else {
			const dict = fetchedDicts.get(lang);
			if (dict) {
				cached = flatten(dict);
			} else {
				cached = {};
				if (!pendingFetches.has(lang) && !failedLocales.has(lang)) {
					downloadLocale(lang);
				}
			}
		}
		flatCache.set(lang, cached);
	}
	if (lang === "en") enFlat = cached;
	return cached;
}

// Pre-cache English for fallback
getFlat("en");

async function loadStoredLocales(): Promise<void> {
	try {
		const storedLocales = await invoke<StoredLocale[]>("load_locales");

		for (const stored of storedLocales) {
			try {
				const data = JSON.parse(stored.data) as LocaleDict;
				activateLocale(stored.code, stored.id, data);
				if (!isBundled(stored.code)) {
					void downloadLocale(stored.code);
				}
			} catch (error) {
				console.error(
					`[i18n] Failed to parse stored locale "${stored.id}":`,
					error,
				);
			}
		}
	} catch (error) {
		console.error("[i18n] Failed to load stored locales:", error);
	}
}

export async function downloadLocale(lang: string): Promise<void> {
	if (isBundled(lang)) return;
	const pending = pendingFetches.get(lang);
	if (pending) return pending;

	const cached = fetchedDicts.get(lang);
	const promise = (async () => {
		const cachedVersion = cached?.version;
		if (typeof cachedVersion === "string") {
			try {
				const res = await fetch(`${API_BASE}/${lang}/version`);
				if (!res.ok) throw new Error(`HTTP ${res.status}`);

				const remote = (await res.json()) as LocaleVersion;
				if (typeof remote.version !== "string") {
					throw new Error("Invalid version response");
				}
				if (remote.version === cachedVersion) {
					failedLocales.delete(lang);
					return;
				}
			} catch (error) {
				console.error(
					`[i18n] Failed to check locale "${lang}" version:`,
					error,
				);
				return;
			}
		}

		i18nLoader.loading = lang;
		try {
			const res = await fetch(`${API_BASE}/${lang}`);
			if (!res.ok) throw new Error(`HTTP ${res.status}`);

			const data = (await res.json()) as LocaleDict;
			const id = data.id;
			if (typeof id !== "string") {
				throw new Error("Invalid locale response");
			}

			activateLocale(lang, id, data);
			invoke("save_locale", { data: JSON.stringify(data) }).catch(
				(error) =>
					console.error("[i18n] Failed to persist locale:", error),
			);
		} catch (error) {
			failedLocales.add(lang);
			console.error(`[i18n] Failed to fetch locale "${lang}":`, error);
		}
	})().finally(() => {
		pendingFetches.delete(lang);
		if (i18nLoader.loading === lang) {
			i18nLoader.loading = null;
		}
	});

	pendingFetches.set(lang, promise);
	return promise;
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

async function fetchAvailableLocales(): Promise<void> {
	try {
		const res = await fetch(`${API_BASE}/locales`);
		if (!res.ok) throw new Error(`HTTP ${res.status}`);
		const remoteLocales = (await res.json()) as LocaleEntry[];
		const remoteIds = new Set(remoteLocales.map((locale) => locale.id));
		const storedOnly = locales.filter(
			(locale) => !remoteIds.has(locale.id),
		);

		locales.splice(0, locales.length, ...remoteLocales, ...storedOnly);
	} catch (error) {
		console.error("[i18n] Failed to fetch available locales:", error);
	}
}

async function initializeI18n(): Promise<void> {
	await loadStoredLocales();
	await fetchAvailableLocales();
}

export { locales };

void initializeI18n();
