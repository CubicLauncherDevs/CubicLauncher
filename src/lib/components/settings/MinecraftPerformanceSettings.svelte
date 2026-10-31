<script lang="ts">
	import { onMount } from "svelte";
	import { launcherStore } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import { getRecommendedRam } from "$lib/api/cubicApi";
	import {
		JVM_PRESETS,
		detectJvmPreset,
		jvmPresetArgs,
		type JvmPreset,
	} from "$lib/utils/performanceProfiles";
	import "./controls.css";

	let { onsave }: { onsave: () => Promise<void> } = $props();

	const MIN_MEMORY_GB = 0.5;
	const MAX_MEMORY_GB = 64;

	let recommendedRam = $state<{
		total_gb: number;
		min_gb: number;
		max_gb: number;
	} | null>(null);
	let ramLoading = $state(false);

	const minGb = $derived(launcherStore.settings.min_memory / 1024);
	const maxGb = $derived(launcherStore.settings.max_memory / 1024);
	const totalGb = $derived(recommendedRam?.total_gb ?? null);
	const usagePercent = $derived(
		totalGb && totalGb > 0
			? Math.min(100, Math.round((maxGb / totalGb) * 100))
			: 0,
	);
	const ramWarning = $derived.by(() => {
		if (totalGb === null) return null;
		if (maxGb >= totalGb - 1)
			return t("settings.minecraft.perf.ramTooHigh", {
				total: totalGb,
			});
		if (maxGb < 1) return t("settings.minecraft.perf.ramTooLow");
		return null;
	});

	const jvmPreset = $derived(
		detectJvmPreset(launcherStore.settings.jvm_args),
	);

	onMount(() => {
		void loadRecommendedRam();
	});

	async function loadRecommendedRam() {
		ramLoading = true;
		try {
			recommendedRam = await getRecommendedRam();
		} finally {
			ramLoading = false;
		}
	}

	async function applyRecommendedRam() {
		if (!recommendedRam) {
			if (ramLoading) return;
			await loadRecommendedRam();
		}
		if (recommendedRam) {
			launcherStore.settings.min_memory = recommendedRam.min_gb * 1024;
			launcherStore.settings.max_memory = recommendedRam.max_gb * 1024;
			await onsave();
		}
	}

	function clampMemoryGb(value: number, min: number, max: number) {
		return Math.round(Math.min(Math.max(value, min), max) * 10) / 10;
	}

	function setMinMemoryGb(value: number) {
		const gb = clampMemoryGb(
			Number.isFinite(value) ? value : minGb,
			MIN_MEMORY_GB,
			Math.min(maxGb, MAX_MEMORY_GB),
		);
		launcherStore.settings.min_memory = Math.round(gb * 1024);
	}

	function setMaxMemoryGb(value: number) {
		const gb = clampMemoryGb(
			Number.isFinite(value) ? value : maxGb,
			Math.max(minGb, MIN_MEMORY_GB),
			MAX_MEMORY_GB,
		);
		launcherStore.settings.max_memory = Math.round(gb * 1024);
	}

	function handleMemoryInput(kind: "min" | "max", event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const value = parseFloat(input.value);
		if (kind === "min") setMinMemoryGb(value);
		else setMaxMemoryGb(value);
		input.value = String(kind === "min" ? minGb : maxGb);
	}

	async function applyJvmPreset(preset: JvmPreset) {
		launcherStore.settings.jvm_args = jvmPresetArgs(preset);
		await onsave();
	}
</script>

<div class="minecraft-performance settings-controls">
	<div class="qm-field-group">
		<div class="qm-field">
			<label for="min-mem">{t("settings.minecraft.minRam")}</label>
			<div class="qm-ram-stepper">
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => setMinMemoryGb(minGb - 0.5)}>−</button
				>
				<input
					type="number"
					id="min-mem"
					class="qm-ram-value"
					step="0.5"
					min="0.5"
					max={maxGb}
					value={minGb}
					onchange={(event) => handleMemoryInput("min", event)}
				/>
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => setMinMemoryGb(minGb + 0.5)}>+</button
				>
			</div>
		</div>
		<div class="qm-field">
			<label for="max-mem">{t("settings.minecraft.maxRam")}</label>
			<div class="qm-ram-stepper">
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => setMaxMemoryGb(maxGb - 0.5)}>−</button
				>
				<input
					type="number"
					id="max-mem"
					class="qm-ram-value"
					step="0.5"
					min={minGb}
					max={MAX_MEMORY_GB}
					value={maxGb}
					onchange={(event) => handleMemoryInput("max", event)}
				/>
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => setMaxMemoryGb(maxGb + 0.5)}>+</button
				>
			</div>
		</div>
	</div>

	<div class="ram-summary">
		<div class="ram-summary-head">
			<span>{t("settings.minecraft.perf.ramUsage")}</span>
			{#if totalGb !== null}
				<span class="ram-summary-total"
					>{t("settings.minecraft.perf.ramTotal", {
						total: totalGb,
					})}</span
				>
			{/if}
		</div>
		<div
			class="ram-bar"
			role="img"
			aria-label={t("settings.minecraft.perf.ramUsage")}
		>
			<span
				class="ram-bar-fill"
				class:warning={ramWarning !== null}
				style={`width: ${usagePercent}%`}
			></span>
		</div>
	</div>

	<div class="qm-recommended-ram-wrapper">
		<button
			type="button"
			class="qm-recommended-ram-btn"
			onclick={applyRecommendedRam}
			disabled={ramLoading || !recommendedRam}
		>
			{t("settings.minecraft.recommendedRamButton")}
		</button>
		{#if ramLoading}
			<span class="qm-recommended-ram-hint">
				{t("settings.minecraft.loadingRecommendedRam")}
			</span>
		{:else if recommendedRam}
			<span class="qm-recommended-ram-hint">
				{t("settings.minecraft.recommendedRamHint", {
					min: recommendedRam.min_gb,
					max: recommendedRam.max_gb,
					total: recommendedRam.total_gb,
				})}
			</span>
		{/if}
	</div>

	{#if ramWarning}
		<p class="ram-warning" role="status">{ramWarning}</p>
	{:else}
		<span class="qm-ram-hint">{t("settings.minecraft.ramHint")}</span>
	{/if}

	<section
		class="jvm-presets"
		aria-label={t("settings.minecraft.perf.jvmPresetsTitle")}
	>
		<h3 class="jvm-presets-title">
			{t("settings.minecraft.perf.jvmPresetsTitle")}
		</h3>
		<p class="jvm-presets-hint">
			{t("settings.minecraft.perf.jvmPresetsHint")}
		</p>
		<div class="jvm-preset-list">
			{#each JVM_PRESETS as preset (preset)}
				<button
					type="button"
					class="jvm-preset"
					class:active={jvmPreset === preset}
					aria-pressed={jvmPreset === preset}
					onclick={() => applyJvmPreset(preset)}
				>
					<span class="jvm-preset-title">
						{t(
							`settings.minecraft.perf.jvmPresets.${preset}.title`,
						)}
					</span>
					<span class="jvm-preset-hint">
						{t(`settings.minecraft.perf.jvmPresets.${preset}.hint`)}
					</span>
				</button>
			{/each}
		</div>
		{#if jvmPreset === "custom"}
			<p class="jvm-custom">
				{t("settings.minecraft.perf.jvmCustom")}
			</p>
		{/if}
	</section>
</div>

<style>
	.qm-field {
		margin-bottom: calc(15px * var(--cubic-interface-gap-factor, 1));
	}

	.qm-field label {
		display: block;
		font-size: 0.8rem;
		color: var(--text-secondary);
		margin-bottom: 6px;
	}

	.minecraft-performance {
		display: flex;
		flex-direction: column;
	}

	.ram-summary {
		margin: 2px 0 12px;
	}

	.ram-summary-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 10px;
		font-size: 0.74rem;
		color: var(--text-secondary);
		margin-bottom: 6px;
	}

	.ram-summary-total {
		color: var(--text-muted);
		font-size: 0.7rem;
	}

	.ram-bar {
		height: 6px;
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		overflow: hidden;
	}

	.ram-bar-fill {
		display: block;
		height: 100%;
		background: var(--accent);
		transition: width 0.2s ease;
	}

	.ram-bar-fill.warning {
		background: var(--warning, #e6a23c);
	}

	.ram-warning {
		margin: 6px 0 0;
		font-size: 0.75rem;
		line-height: 1.5;
		color: var(--warning, #e6a23c);
	}

	.jvm-presets {
		margin-top: 18px;
		padding-top: 16px;
		border-top: 1px solid var(--border-color);
	}

	.jvm-presets-title {
		margin: 0 0 4px;
		font-size: 0.68rem;
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.jvm-presets-hint {
		margin: 0 0 10px;
		font-size: 0.72rem;
		line-height: 1.45;
		color: var(--text-muted);
	}

	.jvm-preset-list {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 8px;
	}

	@media (max-width: 640px) {
		.jvm-preset-list {
			grid-template-columns: 1fr;
		}
	}

	.jvm-preset {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 4px;
		padding: 10px 12px;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-family: inherit;
		text-align: left;
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s,
			color 0.15s;
	}

	.jvm-preset:hover {
		color: var(--text-primary);
		border-color: var(--text-muted);
	}

	.jvm-preset.active {
		background: var(--surface-selected);
		border-color: var(--accent);
		color: var(--text-primary);
		box-shadow: var(--shadow-sm);
	}

	.jvm-preset:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.jvm-preset-title {
		font-size: 0.8rem;
		font-weight: 700;
	}

	.jvm-preset-hint {
		font-size: 0.69rem;
		line-height: 1.4;
		color: var(--text-muted);
	}

	.jvm-custom {
		margin: 8px 0 0;
		font-size: 0.72rem;
		color: var(--text-muted);
	}
	.qm-field-group {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 15px;
	}

	.qm-ram-stepper {
		display: flex;
		align-items: center;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		box-shadow: var(--shadow-inset);
	}

	.qm-stepper-btn {
		background: none;
		border: none;
		color: var(--text-secondary);
		padding: 8px 14px;
		font-size: 1.1rem;
		font-weight: 700;
		cursor: pointer;
		transition: color 0.15s;
		line-height: 1;
	}

	.qm-stepper-btn:hover {
		color: var(--text-primary);
	}

	.qm-ram-value {
		flex: 1;
		min-width: 0;
		text-align: center;
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-primary);
		padding: 8px 4px;
		background: transparent;
		border: none;
		outline: none;
		font-family: var(--font-family);
		-moz-appearance: textfield;
		appearance: textfield;
	}

	.qm-ram-value::-webkit-outer-spin-button,
	.qm-ram-value::-webkit-inner-spin-button {
		-webkit-appearance: none;
		margin: 0;
	}

	.qm-ram-stepper:focus-within {
		border-color: var(--text-muted);
	}

	.qm-ram-stepper .qm-ram-value:focus-visible {
		outline: none;
	}

	.qm-recommended-ram-wrapper {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 4px;
		margin-top: 10px;
	}

	.qm-recommended-ram-btn {
		background: rgba(var(--accent-rgb), 0.08);
		color: var(--accent);
		border: 1px solid rgba(var(--accent-rgb), 0.3);
		padding: 6px 12px;
		border-radius: var(--border-radius-sm);
		font-family: var(--font-family);
		font-size: 0.78rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.15s;
	}

	.qm-recommended-ram-btn:hover:not(:disabled) {
		background: rgba(var(--accent-rgb), 0.15);
	}

	.qm-recommended-ram-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.qm-recommended-ram-hint {
		font-size: 0.7rem;
		color: var(--text-muted);
	}
</style>
