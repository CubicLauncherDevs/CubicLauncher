import { SvelteSet } from "svelte/reactivity";

export type LocaleEntry = {
	code: string;
	id: string;
	label: string;
	flag: string;
};

export const i18nLoader = $state({
	loading: null as string | null,
	fetched: new SvelteSet<string>(),
	dictVersion: {} as Record<string, number>,
});

export const locales = $state<LocaleEntry[]>([
	{ code: "es", id: "es-ES", label: "Español", flag: "🇪🇸" },
	{ code: "en", id: "en-US", label: "English", flag: "🇬🇧" },
	{ code: "fr", id: "fr-FR", label: "Français", flag: "🇫🇷" },
	{ code: "de", id: "de-DE", label: "Deutsch", flag: "🇩🇪" },
	{ code: "uk", id: "uk-UA", label: "Українська", flag: "🇺🇦" },
	{ code: "ja", id: "ja-JP", label: "日本語", flag: "🇯🇵" },
]);
