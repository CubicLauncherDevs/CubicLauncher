import { launcherStore } from "$lib/state/state.svelte";
import es from "./es.json";
import en from "./en.json";
import fr from "./fr.json";
import de from "./de.json";

type DictValue = string | { [key: string]: DictValue };
const dicts: Record<string, DictValue> = { es, en, fr, de };

const flatCache = new Map<string, Record<string, string>>();

function flatten(
	obj: Record<string, DictValue>,
	prefix = "",
): Record<string, string> {
	const result: Record<string, string> = {};
	for (const key in obj) {
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
	let cached = flatCache.get(lang);
	if (!cached) {
		const dict = dicts[lang] || dicts["es"];
		cached = dict && typeof dict === "object" ? flatten(dict) : {};
		flatCache.set(lang, cached);
	}
	return cached;
}

export function t(
	key: string,
	params?: Record<string, string | number>,
): string {
	const lang = launcherStore.settings?.language || "es";
	const flat = getFlat(lang);

	const result = flat[key];
	if (result === undefined) return key;
	if (!params) return result;

	return result.replace(/\{(\w+)\}/g, (_, name) =>
		String(params[name] ?? `{${name}}`),
	);
}
