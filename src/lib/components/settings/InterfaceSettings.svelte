<script lang="ts">
	import Select from "$lib/components/layout/Select.svelte";
	import { launcherStore, showError } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import { applyInterfaceScale } from "$lib/api/interfaceAppearance";
	import {
		DEFAULT_INTERFACE_PREFERENCES,
		INTERFACE_SCALES,
		INTERFACE_DENSITIES,
		interfacePreferences,
		type InterfacePreferences,
	} from "$lib/utils/interfacePreferences";
	import "./controls.css";

	let { onsave }: { onsave: () => Promise<void> } = $props();
	let changing = $state(false);
	let selectedScale = $state("100");
	let selectedDensity = $state("theme");
	const preferences = $derived(
		interfacePreferences(launcherStore.settings.interface_preferences),
	);
	$effect(() => {
		selectedScale = String(preferences.scale);
		selectedDensity = preferences.density;
	});
	const scales = INTERFACE_SCALES.map((value) => ({
		value: String(value),
		label: `${value} %`,
	}));
	const densities = $derived(
		INTERFACE_DENSITIES.map((value) => ({
			value,
			label: t(`settings.interface.densities.${value}`),
		})),
	);

	async function update(patch: Partial<InterfacePreferences>) {
		if (changing) return;
		changing = true;
		try {
			const next = interfacePreferences({ ...preferences, ...patch });
			// Save only once native zoom succeeds; a failure keeps the old setting.
			if (next.scale !== preferences.scale)
				await applyInterfaceScale(next.scale);
			launcherStore.settings.interface_preferences = next;
			await onsave();
		} catch (error) {
			showError(t("settings.interface.scaleError"), String(error));
		} finally {
			selectedScale = String(preferences.scale);
			selectedDensity = preferences.density;
			changing = false;
		}
	}
</script>

<div class="interface-settings settings-controls" aria-busy={changing}>
	<div class="interface-field">
		<Select
			id="interface-scale"
			label={t("settings.interface.scale")}
			bind:value={selectedScale}
			options={scales}
			disabled={changing}
			onchange={(value) =>
				update({
					scale: Number(value) as InterfacePreferences["scale"],
				})}
		/>
		<p class="qm-ram-hint">{t("settings.interface.scaleHint")}</p>
	</div>
	<div class="interface-field">
		<Select
			id="interface-density"
			label={t("settings.interface.density")}
			bind:value={selectedDensity}
			options={densities}
			disabled={changing}
			onchange={(value) =>
				update({ density: value as InterfacePreferences["density"] })}
		/>
		<p class="qm-ram-hint">{t("settings.interface.densityHint")}</p>
	</div>
	<div class="interface-actions">
		<button
			type="button"
			class="detect-btn"
			disabled={changing}
			onclick={() => update(DEFAULT_INTERFACE_PREFERENCES)}
		>
			{t("settings.personalize.reset")}
		</button>
	</div>
</div>

<style>
	.interface-settings {
		display: flex;
		flex-direction: column;
		padding-top: 10px;
		gap: calc(14px * var(--cubic-interface-gap-factor, 1));
	}
	.interface-field {
		min-width: 0;
	}
	.interface-settings :global(.qm-ram-hint) {
		margin: 6px 0 0;
		padding: 0;
	}
	.interface-actions {
		padding-top: 12px;
		border-top: 1px solid var(--border-color);
	}
</style>
