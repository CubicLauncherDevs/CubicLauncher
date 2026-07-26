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

const API_BASE = "https://i18n.cubiclauncher.org";

const bundled: Record<string, LocaleDict> = { es, en };
const fetchedDicts = new Map<string, LocaleDict>();
const flatCache = new Map<string, Record<string, string>>();
const pendingFetches = new Map<string, Promise<void>>();
const failedLocales = new Set<string>();

let enFlat: Record<string, string> | null = null;

// Init reactive state for bundled locales
i18nLoader.fetched.add("es");
i18nLoader.fetched.add("en");

export function isBundled(lang: string): boolean {
	return lang === "es" || lang === "en";
}

export function isFetched(lang: string): boolean {
	return isBundled(lang) || fetchedDicts.has(lang);
}

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
		if (isBundled(lang)) {
			const dict = bundled[lang];
			cached = dict && typeof dict === "object" ? flatten(dict) : {};
		} else {
			void (i18nLoader.dictVersion[lang] ?? 0);
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

async function loadCachedLocales(codes?: string[]): Promise<void> {
	const toLoad = codes ?? locales.filter((loc) => !isBundled(loc.code)).map(l => l.code);

	const results = await Promise.allSettled(
		toLoad.map(async (code) => {
			const dataStr = await invoke<string | null>("load_locale", {
				lang: code,
			});
			if (dataStr) {
				const data = JSON.parse(dataStr) as LocaleDict;
				fetchedDicts.set(code, data);
				flatCache.set(code, flatten(data));
				i18nLoader.fetched.add(code);
				i18nLoader.dictVersion[code] = (i18nLoader.dictVersion[code] ?? 0) + 1;
			}
		}),
	);

	for (const result of results) {
		if (result.status === "rejected") {
			console.error(
				"[i18n] Failed to load cached locale:",
				result.reason,
			);
		}
	}
}

export async function downloadLocale(lang: string): Promise<void> {
	if (isBundled(lang)) return;
	if (pendingFetches.has(lang)) return pendingFetches.get(lang)!;
	if (fetchedDicts.has(lang)) return;

	i18nLoader.loading = lang;

	const promise = fetch(`${API_BASE}/${lang}`)
		.then((res) => {
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			return res.json() as Promise<LocaleDict>;
		})
		.then((data) => {
			fetchedDicts.set(lang, data);
			flatCache.set(lang, flatten(data));
			i18nLoader.fetched.add(lang);
			i18nLoader.dictVersion[lang] = (i18nLoader.dictVersion[lang] ?? 0) + 1;
			invoke("save_locale", { lang, data: JSON.stringify(data) }).catch(
				(e) => console.error("[i18n] Failed to persist locale:", e),
			);
		})
		.catch((err) => {
			failedLocales.add(lang);
			console.error(`[i18n] Failed to fetch locale "${lang}":`, err);
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
		const data = (await res.json()) as LocaleEntry[];

		locales.length = 0;
		locales.push(...data);

		const newCodes = data
			.map((l) => l.code)
			.filter((c) => !isBundled(c));
		if (newCodes.length > 0) {
			await loadCachedLocales(newCodes);
		}
	} catch (err) {
		console.error("[i18n] Failed to fetch available locales:", err);
	}
}

export { locales };

loadCachedLocales();
fetchAvailableLocales();
