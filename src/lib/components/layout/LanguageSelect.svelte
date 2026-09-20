<script lang="ts">
	import { onMount } from "svelte";
	import {
		t,
		locales,
		downloadLocale,
		loadAvailableLocales,
	} from "$lib/i18n";
	import { i18nLoader } from "$lib/i18n/loader.svelte";
	import Select from "./Select.svelte";

	let {
		value = $bindable(),
		id,
		onchange,
	}: {
		value: string;
		id?: string;
		onchange?: (value: string) => void;
	} = $props();

	const options = $derived(
		locales.map((locale) => {
			const key = `languages.${locale.code}`;
			const translated = t(key);
			const label = translated === key ? locale.label : translated;
			return {
				value: locale.code,
				label,
				subtitle: label === locale.label ? undefined : locale.label,
				badge: locale.id,
				status:
					i18nLoader.loading === locale.code
						? ("loading" as const)
						: !i18nLoader.fetched.has(locale.code)
							? ("download" as const)
							: undefined,
			};
		}),
	);

	onMount(() => {
		void loadAvailableLocales();
	});
</script>

<Select
	{id}
	compact
	label={t("settings.launcher.language")}
	{options}
	bind:value
	onchange={(code) => {
		void downloadLocale(code);
		onchange?.(code);
	}}
/>
