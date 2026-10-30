<script lang="ts">
	import { t } from "$lib/i18n";
	import Select from "$lib/components/layout/Select.svelte";

	let {
		useOverrides = $bindable(false),
		selectedJavaVersion = $bindable(""),
		jvmArgs = $bindable(""),
		minMem = $bindable(1),
		maxMem = $bindable(2),
		javaOptions = [] as { value: string; label: string; badge?: string }[],
		saving = false,
		onJavaChange,
	}: {
		useOverrides: boolean;
		selectedJavaVersion: string;
		jvmArgs: string;
		minMem: number;
		maxMem: number;
		javaOptions: { value: string; label: string; badge?: string }[];
		saving: boolean;
		onJavaChange: () => void;
	} = $props();

	const MIN_MEMORY_GB = 0.5;
	const MAX_MEMORY_GB = 64;

	function clampMemoryGb(value: number, minGb: number, maxGb: number) {
		return Math.round(Math.min(Math.max(value, minGb), maxGb) * 10) / 10;
	}

	function setMinMem(value: number) {
		minMem = clampMemoryGb(
			Number.isFinite(value) ? value : minMem,
			MIN_MEMORY_GB,
			maxMem,
		);
	}

	function setMaxMem(value: number) {
		maxMem = clampMemoryGb(
			Number.isFinite(value) ? value : maxMem,
			minMem,
			MAX_MEMORY_GB,
		);
	}

	function handleMemoryInput(kind: "min" | "max", event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const value = parseFloat(input.value);
		if (kind === "min") setMinMem(value);
		else setMaxMem(value);
		input.value = String(kind === "min" ? minMem : maxMem);
	}
</script>

<div class="qm-field-checkbox">
	<input type="checkbox" id="use-overrides" bind:checked={useOverrides} />
	<label for="use-overrides">{t("instanceEditor.useCustomConfig")}</label>
</div>
<fieldset disabled={!useOverrides || saving}>
	<Select
		bind:value={selectedJavaVersion}
		options={javaOptions}
		placeholder={t("instanceEditor.javaVersion")}
		onchange={onJavaChange}
	/>
	<span class="qm-themes-hint">{t("instanceEditor.javaHint")}</span>
	<div class="qm-field">
		<label for="instance-jvm-args">{t("instanceEditor.jvmArgs")}</label>
		<textarea
			id="instance-jvm-args"
			bind:value={jvmArgs}
			placeholder="-XX:+UseG1GC -Dsun.rmi.dgc.server.gcInterval=..."
			class="qm-jvm-args-textarea"></textarea>
		<span class="qm-themes-hint">{t("instanceEditor.jvmArgsHint")}</span>
	</div>
	<div class="qm-field-group">
		<div class="qm-field">
			<label for="min-mem">{t("settings.minecraft.minRam")}</label>
			<div class="qm-ram-stepper">
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => setMinMem(minMem - 0.5)}>−</button
				>
				<input
					type="number"
					id="min-mem"
					class="qm-ram-value"
					step="0.5"
					min="0.5"
					max={maxMem}
					value={minMem}
					onchange={(event) => handleMemoryInput("min", event)}
				/>
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => setMinMem(minMem + 0.5)}>+</button
				>
			</div>
		</div>
		<div class="qm-field">
			<label for="max-mem">{t("settings.minecraft.maxRam")}</label>
			<div class="qm-ram-stepper">
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => setMaxMem(maxMem - 0.5)}>−</button
				>
				<input
					type="number"
					id="max-mem"
					class="qm-ram-value"
					step="0.5"
					min={minMem}
					max={MAX_MEMORY_GB}
					value={maxMem}
					onchange={(event) => handleMemoryInput("max", event)}
				/>
				<button
					type="button"
					class="qm-stepper-btn"
					onclick={() => setMaxMem(maxMem + 0.5)}>+</button
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

	.qm-ram-hint {
		display: block;
		margin-top: 1ch;
		font-size: 0.75rem;
		color: var(--text-muted);
		line-height: 1.5;
		padding: 0 4px;
	}

	.qm-jvm-args-textarea {
		width: 100%;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 8px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.85rem;
		resize: vertical;
		min-height: 60px;
		font-family: var(--font-family-mono);
		box-shadow: var(--shadow-inset);
		box-sizing: border-box;
	}

	.qm-jvm-args-textarea:focus {
		outline: none;
		border-color: var(--text-muted);
	}
</style>
