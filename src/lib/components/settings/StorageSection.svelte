<script lang="ts">
	import { onDestroy } from "svelte";
	import { ask, open } from "@tauri-apps/plugin-dialog";
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import {
		getInstancesDirInfo,
		changeInstancesDir,
		resetInstancesDir,
		cancelInstancesDirChange,
		getSharedDirInfo,
		purgeSharedDir,
		changeSharedDir,
		resetSharedDir,
		type InstancesDirInfo,
		type InstancesMoveProgress,
		type SharedDirInfo,
	} from "$lib/api/storageLocation";

	let info = $state<InstancesDirInfo | null>(null);
	let loading = $state(true);
	let moving = $state(false);
	let cancelling = $state(false);
	let error = $state("");
	let progress = $state<InstancesMoveProgress | null>(null);
	let result = $state<{ moved: number; failed: string[] } | null>(null);
	let destroyed = false;

	let sharedInfo = $state<SharedDirInfo | null>(null);
	let sharedLoading = $state(true);
	let sharedBusy = $state(false);
	let sharedError = $state("");
	let sharedResult = $state("");

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

	async function refreshShared() {
		sharedLoading = true;
		try {
			sharedInfo = await getSharedDirInfo();
		} catch (e) {
			sharedError = String(e);
		} finally {
			sharedLoading = false;
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

	function formatBytes(bytes: number) {
		const unit = Math.min(
			4,
			Math.floor(Math.log2(Math.max(1, bytes)) / 10),
		);
		return `${(bytes / 1024 ** unit).toLocaleString(undefined, { maximumFractionDigits: 1 })} ${["B", "KiB", "MiB", "GiB", "TiB"][unit]}`;
	}

	function sharedUsage(usage: { bytes: number; files: number }) {
		if (usage.files === 0) return t("settings.sharedStorage.empty");
		const size = formatBytes(usage.bytes);
		const files = t("settings.sharedStorage.files", {
			count: usage.files,
		});
		return `${size} · ${files}`;
	}

	async function purgeShared() {
		if (sharedBusy || !sharedInfo) return;
		sharedError = "";
		sharedResult = "";
		try {
			const confirmed = await ask(
				t("settings.sharedStorage.purgeConfirmDesc", {
					size: sharedUsage(sharedInfo.total),
				}),
				{
					title: t("settings.sharedStorage.purgeConfirmTitle"),
					kind: "warning",
					okLabel: t("settings.sharedStorage.purge"),
					cancelLabel: t("common.cancel"),
				},
			);
			if (!confirmed) return;
			sharedBusy = true;
			const usage = await purgeSharedDir();
			sharedResult = t("settings.sharedStorage.purged", {
				size: formatBytes(usage.bytes),
				files: usage.files,
			});
			await refreshShared();
		} catch (e) {
			sharedError = String(e);
		} finally {
			sharedBusy = false;
		}
	}

	async function pickShared() {
		if (sharedBusy) return;
		sharedError = "";
		sharedResult = "";
		try {
			const path = await open({
				directory: true,
				multiple: false,
				title: t("settings.sharedStorage.pickTitle"),
			});
			if (typeof path !== "string" || destroyed) return;
			const confirmed = await ask(
				t("settings.sharedStorage.changeConfirmDesc", {
					size: sharedInfo ? sharedUsage(sharedInfo.total) : "0",
				}),
				{
					title: t("settings.sharedStorage.changeConfirmTitle"),
					kind: "warning",
					okLabel: t("settings.sharedStorage.change"),
					cancelLabel: t("common.cancel"),
				},
			);
			if (!confirmed) return;
			sharedBusy = true;
			const usage = await changeSharedDir(path);
			sharedResult = t("settings.sharedStorage.changed", {
				size: formatBytes(usage.bytes),
			});
			await refreshShared();
		} catch (e) {
			sharedError = String(e);
		} finally {
			sharedBusy = false;
		}
	}

	async function resetShared() {
		if (sharedBusy) return;
		sharedError = "";
		sharedResult = "";
		try {
			sharedBusy = true;
			const usage = await resetSharedDir();
			sharedResult = t("settings.sharedStorage.changed", {
				size: formatBytes(usage.bytes),
			});
			await refreshShared();
		} catch (e) {
			sharedError = String(e);
		} finally {
			sharedBusy = false;
		}
	}

	onDestroy(() => {
		destroyed = true;
	});

	void refresh();
	void refreshShared();
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

	<hr class="divider" />

	<h4 class="subsection">{t("settings.sharedStorage.title")}</h4>
	<p class="hint">{t("settings.sharedStorage.description")}</p>

	{#if sharedLoading}
		<p class="hint">{t("common.loading")}</p>
	{:else if sharedInfo}
		<div class="row">
			<span class="label">{t("settings.sharedStorage.currentDir")}</span>
			<code class="path" title={sharedInfo.current_dir}
				>{sharedInfo.current_dir}</code
			>
		</div>
		<div class="row">
			<span class="label">{t("settings.sharedStorage.usage")}</span>
			<span class="value">{sharedUsage(sharedInfo.total)}</span>
		</div>
		{#if sharedInfo.breakdown.length > 0}
			<ul class="usage">
				{#each sharedInfo.breakdown as [name, usage] (name)}
					<li>
						<span class="value">{name}</span>
						<span class="usage-detail"
							>{formatBytes(usage.bytes)} · {usage.files}</span
						>
					</li>
				{/each}
			</ul>
		{/if}

		<div class="actions">
			<button
				type="button"
				class="btn danger"
				disabled={sharedBusy}
				onclick={purgeShared}
			>
				<Icon name="ui:trash" size={16} />
				{t("settings.sharedStorage.purge")}
			</button>
			<button
				type="button"
				class="btn"
				disabled={sharedBusy}
				onclick={pickShared}
			>
				<Icon name="instance:folder" size={16} />
				{t("settings.sharedStorage.change")}
			</button>
			{#if sharedInfo.custom_dir}
				<button
					type="button"
					class="btn secondary"
					disabled={sharedBusy}
					onclick={resetShared}
				>
					<Icon name="ui:refresh" size={16} />
					{t("settings.sharedStorage.reset")}
				</button>
			{/if}
		</div>
	{/if}

	{#if sharedBusy}
		<p class="hint" role="status">{t("settings.sharedStorage.purging")}</p>
	{:else if sharedResult}
		<p class="ok" role="status">{sharedResult}</p>
	{/if}
	{#if sharedError}<p class="error" role="alert">{sharedError}</p>{/if}
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
	.btn.danger:hover:not(:disabled) {
		border-color: var(--color-error);
		color: var(--color-error);
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
		font-size: var(--font-size-sm, 0.8rem);
		margin: 0;
	}
	.divider {
		border: none;
		border-top: 1px solid var(--border);
		margin: 4px 0;
	}
	.subsection {
		font-size: var(--font-size-sm, 0.8rem);
		font-weight: 600;
		color: var(--text-primary);
		margin: 0;
	}
	.usage {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin: 0;
		padding: 0 0 0 12px;
		list-style: none;
	}
	.usage li {
		display: flex;
		justify-content: space-between;
		gap: 12px;
	}
	.usage-detail {
		font-size: var(--font-size-label, 0.72rem);
		color: var(--text-muted);
		white-space: nowrap;
	}
	button:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
</style>
