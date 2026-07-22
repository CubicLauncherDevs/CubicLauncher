<script lang="ts">
	import { t, locales, downloadLocale } from "$lib/i18n";
	import { launcherStore } from "$lib/state/state.svelte";
	import { saveSettings } from "$lib/api/launcherService";
	import Select from "$lib/components/layout/Select.svelte";

	let {
		stepKey,
		isFirstStep,
	}: {
		stepKey: string;
		isFirstStep: boolean;
	} = $props();

	const languageOptions = locales.map((l) => ({
		value: l.code,
		label: l.label,
	}));

	async function onLanguageChange() {
		downloadLocale(launcherStore.settings.language);
		await saveSettings();
	}
</script>

<div class="tut-body">
	<h3 class="tut-title">
		{t(`tutorial.${stepKey}.title`)}
	</h3>
	<p class="tut-desc">
		{t(`tutorial.${stepKey}.desc`)}
	</p>
	{#if isFirstStep}
		<div class="tut-lang">
			<Select
				bind:value={launcherStore.settings.language}
				options={languageOptions}
				onchange={onLanguageChange}
			/>
		</div>
	{/if}
</div>
