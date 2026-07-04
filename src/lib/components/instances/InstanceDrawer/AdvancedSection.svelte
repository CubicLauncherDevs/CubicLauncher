<script lang="ts">
	import { t } from "$lib/i18n";
	import Select from "$lib/components/layout/Select.svelte";

	let {
		useOverrides = $bindable(false),
		selectedJavaVersion = $bindable(""),
		minMem = $bindable(1),
		maxMem = $bindable(2),
		javaOptions = [] as { value: string; label: string; badge?: string }[],
		saving = false,
		onJavaChange,
	}: {
		useOverrides: boolean;
		selectedJavaVersion: string;
		minMem: number;
		maxMem: number;
		javaOptions: { value: string; label: string; badge?: string }[];
		saving: boolean;
		onJavaChange: () => void;
	} = $props();
</script>

<div class="qm-field-checkbox">
	<input
		type="checkbox"
		id="use-overrides"
		bind:checked={useOverrides}
	/>
	<label for="use-overrides">Usar configuración personalizada</label>
</div>
<fieldset disabled={!useOverrides}>
	<Select
		bind:value={selectedJavaVersion}
		options={javaOptions}
		label={t("instanceEditor.javaVersion")}
		onchange={onJavaChange}
	/>
	<span class="qm-themes-hint">{t("instanceEditor.javaHint")}</span>
	<div class="qm-field-group">
		<div class="qm-field">
			<label for="min-mem">{t("settings.minecraft.minRam")}</label>
			<div class="qm-ram-stepper">
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => {
						const v = minMem - 0.5;
						if (v >= 0.5) minMem = v;
					}}>−</button
				>
				<span class="qm-ram-value">{minMem} GB</span>
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => {
						const v = minMem + 0.5;
						if (v <= maxMem) minMem = v;
					}}>+</button
				>
			</div>
		</div>
		<div class="qm-field">
			<label for="max-mem">{t("settings.minecraft.maxRam")}</label>
			<div class="qm-ram-stepper">
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => {
						const v = maxMem - 0.5;
						if (v >= minMem) maxMem = v;
					}}>−</button
				>
				<span class="qm-ram-value">{maxMem} GB</span>
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => {
						const v = maxMem + 0.5;
						if (v <= 64) maxMem = v;
					}}>+</button
				>
			</div>
		</div>
	</div>
	<span class="qm-ram-hint">{t("settings.minecraft.ramHint")}</span>
</fieldset>

<style>
	.qm-field-checkbox {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 12px;
		margin-top: 8px;
		cursor: pointer;
		user-select: none;
	}

	.qm-field-checkbox input[type="checkbox"] {
		appearance: none;
		-webkit-appearance: none;
		width: 18px;
		height: 18px;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		position: relative;
		transition: all 0.2s;
	}

	.qm-field-checkbox input[type="checkbox"]:checked {
		background: var(--accent);
		border-color: var(--accent);
	}

	.qm-field-checkbox input[type="checkbox"]:checked::after {
		content: "✓";
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		color: var(--accent-text);
		font-size: 11px;
		font-weight: 800;
	}

	.qm-field-checkbox label {
		font-size: 0.85rem;
		color: var(--text-secondary);
		cursor: pointer;
		transition: color 0.2s;
	}

	.qm-field-checkbox:hover label {
		color: var(--text-primary);
	}

	.qm-field-checkbox input[type="checkbox"]:hover {
		border-color: var(--text-muted);
	}

	fieldset:disabled {
		border: none;
		padding: 0;
		margin: 0;
		opacity: 0.45;
		pointer-events: none;
	}
	fieldset {
		border: none;
	}

	.qm-themes-hint {
		display: block;
		margin-top: 8px;
		font-size: 0.75rem;
		color: var(--text-secondary);
		line-height: 1.4;
		cursor: pointer;
		transition: color 0.2s;
	}

	.qm-themes-hint:hover {
		color: var(--text-primary);
	}

	.qm-field {
		margin-top: 1vw;
	}

	.qm-field label {
		display: block;
		font-size: 0.8rem;
		color: var(--text-secondary);
		margin-bottom: 6px;
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
		box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.25);
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
		text-align: center;
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-primary);
		padding: 8px 4px;
		user-select: none;
	}

	.qm-ram-hint {
		display: block;
		margin-top: 1ch;
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.5;
		padding: 0 4px;
	}
</style>
