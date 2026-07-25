import { SvelteSet } from "svelte/reactivity";

export const i18nLoader = $state({
	version: 0,
	loading: null as string | null,
	fetched: new SvelteSet<string>(),
});
