<script lang="ts">
	import { launcherStore } from "$lib/state/state.svelte";
	import { t } from "$lib/i18n";
	import {
		LAUNCHER_PERFORMANCE_PRESETS,
		PERFORMANCE_FLAG_KEYS,
		detectLauncherPreset,
		launcherPresetFlags,
		type LauncherPerformancePreset,
		type PerformanceFlagKey,
		type PerformanceFlags,
	} from "$lib/utils/performanceProfiles";
	import "./controls.css";

	let { onsave }: { onsave: () => Promise<void> } = $props();

	const flags = $derived<PerformanceFlags>({
		reduce_animations: launcherStore.settings.reduce_animations,
		disable_blur_effects: launcherStore.settings.disable_blur_effects,
		disable_infinite_animations:
			launcherStore.settings.disable_infinite_animations,
		disable_skin3d_animations:
			launcherStore.settings.disable_skin3d_animations,
		reduce_log_animations: launcherStore.settings.reduce_log_animations,
		disable_background_image:
			launcherStore.settings.disable_background_image,
		reduce_skin3d_quality: launcherStore.settings.reduce_skin3d_quality,
	});

	const selectedPreset = $derived(detectLauncherPreset(flags));
	const enabledCount = $derived(
		PERFORMANCE_FLAG_KEYS.filter((key) => flags[key]).length,
	);

	interface PerformanceOption {
		key: PerformanceFlagKey;
		label: string;
		hint: string;
	}

	interface PerformanceGroup {
		id: string;
		title: string;
		options: PerformanceOption[];
	}

	const groups = $derived<PerformanceGroup[]>([
		{
			id: "motion",
			title: t("settings.launcher.performance.groupMotion"),
			options: [
				{
					key: "reduce_animations",
					label: t("settings.launcher.reduceAnimations"),
					hint: t(
						"settings.launcher.performance.reduceAnimationsHint",
					),
				},
				{
					key: "disable_infinite_animations",
					label: t("settings.launcher.disableInfiniteAnimations"),
					hint: t(
						"settings.launcher.performance.disableInfiniteAnimationsHint",
					),
				},
				{
					key: "reduce_log_animations",
					label: t("settings.launcher.reduceLogAnimations"),
					hint: t(
						"settings.launcher.performance.reduceLogAnimationsHint",
					),
				},
			],
		},
		{
			id: "visuals",
			title: t("settings.launcher.performance.groupVisuals"),
			options: [
				{
					key: "disable_blur_effects",
					label: t("settings.launcher.disableBlurEffects"),
					hint: t(
						"settings.launcher.performance.disableBlurEffectsHint",
					),
				},
				{
					key: "disable_background_image",
					label: t(
						"settings.launcher.performance.disableBackgroundImage",
					),
					hint: t(
						"settings.launcher.performance.disableBackgroundImageHint",
					),
				},
			],
		},
		{
			id: "skin",
			title: t("settings.launcher.performance.groupSkin"),
			options: [
				{
					key: "disable_skin3d_animations",
					label: t("settings.launcher.disableSkin3dAnimations"),
					hint: t(
						"settings.launcher.performance.disableSkin3dAnimationsHint",
					),
				},
				{
					key: "reduce_skin3d_quality",
					label: t(
						"settings.launcher.performance.reduceSkin3dQuality",
					),
					hint: t(
						"settings.launcher.performance.reduceSkin3dQualityHint",
					),
				},
			],
		},
	]);

	async function applyPreset(preset: LauncherPerformancePreset) {
		const next = launcherPresetFlags(preset);
		for (const key of PERFORMANCE_FLAG_KEYS) {
			launcherStore.settings[key] = next[key];
		}
		await onsave();
	}

	async function toggle(key: PerformanceFlagKey, event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		launcherStore.settings[key] = input.checked;
		await onsave();
	}
</script>

<div class="performance-settings settings-controls">
	<div class="perf-summary">
		<div>
			<span class="perf-summary-title">
				{t("settings.launcher.performance.presetsTitle")}
			</span>
			<span class="perf-summary-detail">
				{selectedPreset === "custom"
					? t("settings.launcher.performance.custom")
					: t(
							`settings.launcher.performance.presets.${selectedPreset}.title`,
						)}
				· {t("settings.launcher.performance.enabledCount", {
					count: enabledCount,
				})}
			</span>
		</div>
		<button
			type="button"
			class="perf-reset"
			onclick={() => applyPreset("quality")}
			disabled={enabledCount === 0}
		>
			{t("settings.launcher.performance.reset")}
		</button>
	</div>

	<div
		class="perf-presets"
		role="group"
		aria-label={t("settings.launcher.performance.presetsTitle")}
	>
		{#each LAUNCHER_PERFORMANCE_PRESETS as preset (preset)}
			<button
				type="button"
				class="perf-preset"
				class:active={selectedPreset === preset}
				aria-pressed={selectedPreset === preset}
				onclick={() => applyPreset(preset)}
			>
				<span
					class="perf-preset-indicator"
					class:on={selectedPreset === preset}
				></span>
				<span class="perf-preset-title">
					{t(`settings.launcher.performance.presets.${preset}.title`)}
				</span>
				<span class="perf-preset-hint">
					{t(`settings.launcher.performance.presets.${preset}.hint`)}
				</span>
			</button>
		{/each}
	</div>

	{#each groups as group (group.id)}
		<section class="perf-group" aria-label={group.title}>
			<h3 class="perf-group-title">{group.title}</h3>
			{#each group.options as option (option.key)}
				<label class="perf-option">
					<input
						type="checkbox"
						checked={flags[option.key]}
						onchange={(event) => void toggle(option.key, event)}
					/>
					<span class="perf-option-copy">
						<span class="perf-option-label">{option.label}</span>
						<span class="perf-option-hint">{option.hint}</span>
					</span>
				</label>
			{/each}
		</section>
	{/each}

	<p class="qm-ram-hint perf-footnote">
		{t("settings.launcher.performance.osMotionNote")}
	</p>
</div>

<style>
	.performance-settings {
		display: flex;
		flex-direction: column;
		gap: calc(14px * var(--cubic-interface-gap-factor, 1));
		padding-top: 10px;
	}

	.perf-summary {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}

	.perf-summary-title {
		display: block;
		font-size: 0.8rem;
		font-weight: 700;
		color: var(--text-primary);
	}

	.perf-summary-detail {
		display: block;
		margin-top: 3px;
		font-size: 0.72rem;
		color: var(--text-muted);
	}

	.perf-reset {
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-secondary);
		padding: 6px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.72rem;
		font-family: inherit;
		font-weight: 600;
		cursor: pointer;
		transition:
			color 0.15s,
			border-color 0.15s;
	}

	.perf-reset:hover:not(:disabled) {
		color: var(--text-primary);
		border-color: var(--text-muted);
	}

	.perf-reset:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.perf-presets {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 8px;
	}

	@media (max-width: 640px) {
		.perf-presets {
			grid-template-columns: 1fr;
		}
	}

	.perf-preset {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 4px;
		padding: 10px 12px;
		min-height: 78px;
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

	.perf-preset:hover {
		color: var(--text-primary);
		border-color: var(--text-muted);
	}

	.perf-preset.active {
		background: var(--surface-selected);
		border-color: var(--accent);
		color: var(--text-primary);
		box-shadow: var(--shadow-sm);
	}

	.perf-preset:focus-visible,
	.perf-option input:focus-visible,
	.perf-reset:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.perf-preset-indicator {
		position: absolute;
		top: 10px;
		right: 10px;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--bg-card);
		border: 1px solid var(--border-color);
	}

	.perf-preset-indicator.on {
		background: var(--accent);
		border-color: var(--accent);
	}

	.perf-preset-title {
		font-size: 0.82rem;
		font-weight: 700;
		padding-right: 16px;
	}

	.perf-preset-hint {
		font-size: 0.7rem;
		line-height: 1.4;
		color: var(--text-muted);
	}

	.perf-group {
		padding-top: 2px;
	}

	.perf-group-title {
		margin: 0 0 4px;
		font-size: 0.68rem;
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.perf-option {
		display: flex;
		align-items: flex-start;
		gap: 12px;
		padding: 8px 0;
		cursor: pointer;
		user-select: none;
	}

	.perf-option input[type="checkbox"] {
		appearance: none;
		-webkit-appearance: none;
		width: 18px;
		height: 18px;
		margin-top: 1px;
		flex-shrink: 0;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		position: relative;
		transition:
			background 0.2s,
			border-color 0.2s;
	}

	.perf-option input[type="checkbox"]:checked {
		background: var(--accent);
		border-color: var(--accent);
	}

	.perf-option input[type="checkbox"]:checked::after {
		content: "✓";
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		color: var(--accent-text);
		font-size: 11px;
		font-weight: 800;
	}

	.perf-option input[type="checkbox"]:hover {
		border-color: var(--text-muted);
	}

	.perf-option-copy {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.perf-option-label {
		font-size: 0.82rem;
		color: var(--text-secondary);
		transition: color 0.2s;
	}

	.perf-option:hover .perf-option-label {
		color: var(--text-primary);
	}

	.perf-option-hint {
		font-size: 0.7rem;
		line-height: 1.45;
		color: var(--text-muted);
	}

	.perf-footnote {
		margin: 0;
		padding: 0;
	}
</style>
