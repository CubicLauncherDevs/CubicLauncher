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

export const locales = $state<LocaleEntry[]>([]);
