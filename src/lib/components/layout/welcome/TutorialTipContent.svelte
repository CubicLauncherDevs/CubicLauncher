<script lang="ts">
	import { t, locales } from "$lib/i18n";
	import { launcherStore } from "$lib/state/state.svelte";
	import { saveSettings } from "$lib/api/launcherService";

	let {
		stepKey,
		isFirstStep,
	}: {
		stepKey: string;
		isFirstStep: boolean;
	} = $props();

	async function setLanguage(lang: string) {
		launcherStore.settings.language = lang;
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
			{#each locales.filter((l) => l.code === "en" || l.code === "es") as lang}
				<button
					type="button"
					class="tut-lang-btn"
					class:active={launcherStore.settings.language === lang.code}
					onclick={() => setLanguage(lang.code)}>{lang.label}</button
				>
			{/each}
		</div>
	{/if}
</div>
