<script lang="ts">
	import { t, locales, downloadLocale } from "$lib/i18n";
	import { launcherStore } from "$lib/state/state.svelte";
	import { saveSettings } from "$lib/api/launcherService";
	import { openUrl } from "$lib/api/cubicApi";
	import Icon from "$lib/icons/Icon.svelte";
	import Select from "$lib/components/layout/Select.svelte";

	let {
		stepKey,
		isFirstStep,
		isLicenseStep,
	}: {
		stepKey: string;
		isFirstStep: boolean;
		isLicenseStep: boolean;
	} = $props();

	const languageOptions = $derived(
		locales.map((l) => ({
			value: l.code,
			label: l.label,
		})),
	);

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
	{#if isLicenseStep}
		<div class="license-card">
			<p class="license-card-title">{t("tutorial.licenseName")}</p>
			<ul class="license-bullets">
				<li>{t("tutorial.licenseBullet1")}</li>
				<li>{t("tutorial.licenseBullet2")}</li>
				<li>{t("tutorial.licenseBullet3")}</li>
			</ul>
			<button
				type="button"
				class="license-link"
				onclick={() =>
					openUrl("https://www.gnu.org/licenses/gpl-3.0.html")}
			>
				<Icon name="instance:external-link" size={14} />
				{t("tutorial.viewLicense")}
			</button>
		</div>
		<label class="license-check">
			<input
				type="checkbox"
				bind:checked={launcherStore.settings.license_accepted}
			/>
			<span class="license-check-text">
				{t("tutorial.acceptLicense")}
			</span>
		</label>
		<p class="license-note">{t("tutorial.acceptLicenseNote")}</p>
	{/if}
</div>
