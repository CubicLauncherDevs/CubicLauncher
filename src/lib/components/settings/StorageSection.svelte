<script lang="ts">
	import { onDestroy } from "svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import {
		getInstancesDirInfo,
		changeInstancesDir,
		resetInstancesDir,
		cancelInstancesDirChange,
		type InstancesDirInfo,
		type InstancesMoveProgress,
	} from "$lib/api/storageLocation";

	let info = $state<InstancesDirInfo | null>(null);
	let loading = $state(true);
	let moving = $state(false);
	let cancelling = $state(false);
	let error = $state("");
	let progress = $state<InstancesMoveProgress | null>(null);
	let result = $state<{ moved: number; failed: string[] } | null>(null);
	let destroyed = false;

	const percent = $derived(
		progress && progress.bytes_total > 0
			? Math.min(
					100,
					Math.round(
						(progress.bytes_current / progress.bytes_total) * 100,
					),
				)
			: 0,
	);

	async function refresh() {
		loading = true;
		try {
			info = await getInstancesDirInfo();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function pick() {
		if (moving) return;
		error = "";
		try {
			const path = await open({
				directory: true,
				multiple: false,
				title: t("settings.storage.pickTitle"),
			});
			if (typeof path === "string" && !destroyed) {
				await move(() => changeInstancesDir(path, onProgress));
			}
		} catch (e) {
			error = String(e);
		}
	}

	function reset() {
		if (moving) return;
		error = "";
		void move(() => resetInstancesDir(onProgress));
	}

	function onProgress(value: InstancesMoveProgress) {
		if (!destroyed) progress = value;
	}

	async function move(
		operation: () => Promise<{ moved: number; failed: string[] }>,
	) {
		moving = true;
		progress = null;
		result = null;
		error = "";
		try {
			result = await operation();
			await refresh();
		} catch (e) {
			error = String(e);
		} finally {
			if (!destroyed) {
				moving = false;
				progress = null;
			}
		}
	}

	async function cancel() {
		if (cancelling) return;
		cancelling = true;
		try {
			await cancelInstancesDirChange();
		} finally {
			cancelling = false;
		}
	}

	onDestroy(() => {
		destroyed = true;
	});

	void refresh();
</script>

<div class="storage">
	<p class="hint">{t("settings.storage.description")}</p>

	{#if loading}
		<p class="hint">{t("common.loading")}</p>
	{:else if info}
		<div class="row">
			<span class="label">{t("settings.storage.currentDir")}</span>
			<code class="path" title={info.current_dir}>{info.current_dir}</code
			>
		</div>
		<div class="row">
			<span class="label">{t("settings.storage.instanceCount")}</span>
			<span class="value">{info.instance_count}</span>
		</div>

		<div class="actions">
			<button type="button" class="btn" disabled={moving} onclick={pick}>
				<Icon name="instance:folder" size={16} />
				{t("settings.storage.change")}
			</button>
			{#if info.custom_dir}
				<button
					type="button"
					class="btn secondary"
					disabled={moving}
					onclick={reset}
				>
					<Icon name="ui:refresh" size={16} />
					{t("settings.storage.reset")}
				</button>
			{/if}
		</div>
	{/if}

	{#if error}<p class="error" role="alert">{error}</p>{/if}

	{#if moving}
		<div class="progress" role="status" aria-live="polite">
			{#if progress}
				<strong>
					{t("settings.storage.progress", {
						index: progress.instance_index,
						count: progress.instance_count,
					})}
					· {progress.instance_name}
				</strong>
				<span>
					{progress.strategy === "copy"
						? t("settings.storage.copyingFallback")
						: t("settings.storage.linking")}
					{progress.bytes_total > 0 ? ` · ${percent}%` : ""}
				</span>
				<progress
					max="100"
					value={progress.bytes_total > 0 ? percent : undefined}
					aria-label={t("settings.storage.linking")}
				></progress>
				{#if progress.file}<small class="path">{progress.file}</small
					>{/if}
			{:else}
				<span>{t("settings.storage.preparing")}</span>
			{/if}
			<button
				type="button"
				class="btn secondary"
				disabled={cancelling}
				onclick={cancel}
			>
				{cancelling
					? t("settings.storage.cancelling")
					: t("common.cancel")}
			</button>
		</div>
	{:else if result}
		<div class="result" role="status">
			{#if result.failed.length === 0}
				<span class="ok"
					>{t("settings.storage.done", { count: result.moved })}</span
				>
			{:else}
				<span class="error">
					{t("settings.storage.partial", {
						failed: result.failed.length,
					})}
				</span>
				{#each result.failed as failure (failure)}
					<small class="path">{failure}</small>
				{/each}
			{/if}
		</div>
	{/if}
</div>

<style>
	.storage {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.hint {
		color: var(--text-secondary);
		font-size: var(--font-size-sm, 0.8rem);
		line-height: 1.5;
		margin: 0;
	}
	.row {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.label {
		font-size: var(--font-size-label, 0.72rem);
		color: var(--text-secondary);
	}
	.value {
		font-size: var(--font-size-sm, 0.8rem);
		color: var(--text-primary);
	}
	.path {
		overflow-wrap: anywhere;
		color: var(--text-muted);
		font-size: var(--font-size-label, 0.72rem);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
	.btn {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		font: inherit;
		font-size: 0.78rem;
		cursor: pointer;
	}
	.btn:hover:not(:disabled) {
		background: var(--surface-hover);
		border-color: var(--accent);
	}
	.btn.secondary {
		opacity: 0.9;
	}
	.btn:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
	.btn:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.progress,
	.result {
		display: flex;
		flex-direction: column;
		gap: 8px;
		font-size: var(--font-size-sm, 0.8rem);
	}
	.progress strong {
		font-size: var(--font-size-sm, 0.8rem);
	}
	progress {
		width: 100%;
		height: 7px;
		accent-color: var(--accent);
	}
	.error {
		color: var(--color-error);
		font-size: var(--font-size-sm);
		overflow-wrap: anywhere;
		margin: 0;
	}
	.ok {
		color: var(--accent);
	}
	button:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
</style>
