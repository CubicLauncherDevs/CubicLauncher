<script lang="ts">
	import { t } from "#lib/i18n/index.js";

	interface Props {
		initial: Record<string, string>;
		onchange: (vars: Record<string, string>) => void;
	}

	let { initial, onchange }: Props = $props();

	let entries = $state<Array<{ key: string; value: string }>>([]);

	function init() {
		const keys = Object.keys(initial);
		entries =
			keys.length > 0
				? keys.map((k) => ({ key: k, value: initial[k] }))
				: [{ key: "", value: "" }];
	}

	function sync() {
		const record: Record<string, string> = {};
		for (const entry of entries) {
			if (entry.key.trim() !== "") {
				record[entry.key.trim()] = entry.value;
			}
		}
		onchange(record);
	}

	function addEntry() {
		entries = [...entries, { key: "", value: "" }];
	}

	function removeEntry(index: number) {
		entries = entries.filter((_, i) => i !== index);
		sync();
	}

	init();
</script>

<span class="env-label">{t("settings.java.envVars")}</span>
{#each entries as entry, i (entry)}
	<div class="env-row">
		<input
			type="text"
			value={entry.key}
			placeholder="KEY"
			oninput={(e) => {
				entries[i].key = e.currentTarget.value;
				sync();
			}}
			class="env-input"
		/>
		<span class="env-eq">=</span>
		<input
			type="text"
			value={entry.value}
			placeholder="VALUE"
			oninput={(e) => {
				entries[i].value = e.currentTarget.value;
				sync();
			}}
			class="env-input"
		/>
		<button
			type="button"
			class="env-remove-btn"
			onclick={() => removeEntry(i)}>✕</button
		>
	</div>
{/each}
<button type="button" class="env-add-btn" onclick={addEntry}
	>+ {t("settings.java.envVarsAdd")}</button
>

<style>
	.env-label {
		display: block;
		margin-bottom: 8px;
		color: var(--text-secondary);
		font-size: 0.8rem;
	}
	.env-row {
		display: flex;
		gap: 4px;
		align-items: center;
		margin-bottom: 4px;
	}
	.env-input {
		flex: 1;
		min-width: 0;
		width: 0;
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 4px 8px;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		height: 28px;
		box-shadow: var(--shadow-inset);
		box-sizing: border-box;
	}
	.env-input:focus {
		outline: none;
		border-color: var(--text-muted);
	}
	.env-eq {
		color: var(--text-muted);
		font-size: 0.8rem;
		flex-shrink: 0;
	}
	.env-remove-btn {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		padding: 2px;
		font-size: 1rem;
		line-height: 1;
		flex-shrink: 0;
	}
	.env-remove-btn:hover {
		color: var(--color-error);
	}
	.env-add-btn {
		background: none;
		border: 1px dashed var(--border-color);
		color: var(--text-secondary);
		cursor: pointer;
		padding: 4px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		margin-top: 2px;
	}
	.env-add-btn:hover {
		border-color: var(--text-muted);
		color: var(--text-primary);
	}
</style>
