<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import { openInstanceDir } from "$lib/api/cubicApi";
	import { showErrorParsed } from "$lib/state/state.svelte";
	import type {
		createMarketState,
		LocalStatusFilter,
	} from "$lib/state/marketState.svelte";
	import type { ContentType } from "$lib/types/market";
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";

	let {
		market,
		instanceId,
		contentType,
		view = $bindable("list"),
		onDelete,
	}: {
		market: ReturnType<typeof createMarketState>;
		instanceId: string;
		contentType: ContentType;
		view: "list" | "cards";
		onDelete: (filenames: string[]) => void;
	} = $props();
	let choosingFiles = $state(false);
	let disposed = false;
	const busy = $derived(
		market.localOperationBusy || market.instanceBusy || choosingFiles,
	);
	const report = $derived(market.localOperationReport);
	const statusOptions = $derived<
		{ value: LocalStatusFilter; label: string }[]
	>([
		{ value: "all", label: t("market.manage.allStates") },
		{ value: "enabled", label: t("market.manage.enabled") },
		{ value: "disabled", label: t("market.manage.disabled") },
	]);

	onMount(() => {
		try {
			view =
				localStorage.getItem("marketInstalledView") === "cards"
					? "cards"
					: "list";
		} catch {
			/* Storage unavailable */
		}
	});
	onDestroy(() => {
		disposed = true;
	});
	function setView(value: "list" | "cards") {
		view = value;
		try {
			localStorage.setItem("marketInstalledView", value);
		} catch {
			/* Storage unavailable */
		}
	}
	async function addFiles() {
		if (busy) return;
		choosingFiles = true;
		try {
			const selected = await open({
				multiple: true,
				directory: false,
				filters: [
					{
						name: t("market.filter.tabLocal"),
						extensions:
							contentType === "mods"
								? ["jar", "disabled"]
								: ["zip"],
					},
				],
			});
			if (!disposed && selected)
				await market.importLocal(
					Array.isArray(selected) ? selected : [selected],
				);
		} catch (error) {
			if (!disposed) showErrorParsed(error);
		} finally {
			choosingFiles = false;
		}
	}

	async function copyFailures() {
		if (!report) return;
		try {
			await navigator.clipboard.writeText(
				report.failures
					.map(({ filename, error }) => `${filename}: ${error}`)
					.join("\n"),
			);
		} catch (error) {
			if (!disposed) showErrorParsed(error);
		}
	}
</script>

<div class="installed-toolbar">
	<div class="tools">
		<button
			type="button"
			class="primary"
			disabled={busy || market.loading}
			onclick={addFiles}
			><Icon name="ui:upload" size={15} />{t(
				"market.manage.addFiles",
			)}</button
		>
		<button
			type="button"
			onclick={() => openInstanceDir(instanceId, contentType)}
			><Icon name="instance:folder" size={15} />{t(
				"market.manage.openFolder",
			)}</button
		>
		<button
			type="button"
			disabled={market.localOperationBusy || market.loading}
			onclick={market.refresh}
			><Icon name="ui:refresh" size={15} />{t(
				"market.manage.refresh",
			)}</button
		>
		<div
			class="view-options segmented-control"
			role="group"
			aria-label={t("market.manage.view")}
		>
			<button
				type="button"
				aria-pressed={view === "list"}
				onclick={() => setView("list")}
				><Icon name="log:logs" size={15} />{t(
					"market.manage.list",
				)}</button
			>
			<button
				type="button"
				aria-pressed={view === "cards"}
				onclick={() => setView("cards")}
				><Icon name="instance:grid" size={15} />{t(
					"market.manage.cards",
				)}</button
			>
		</div>
	</div>
	<div class="selection-tools">
		{#if contentType === "mods"}
			<div
				class="status-filter segmented-control"
				role="group"
				aria-label={t("market.manage.status")}
			>
				{#each statusOptions as option (option.value)}
					<button
						type="button"
						aria-pressed={market.filters.localStatus ===
							option.value}
						disabled={market.localOperationBusy}
						onclick={() => market.setLocalStatus(option.value)}
						>{option.label}</button
					>
				{/each}
			</div>
		{/if}
		<button
			type="button"
			disabled={busy || market.loading || !market.items.length}
			onclick={market.selectAllLocal}
			><Icon name="instance:check-square" size={15} />{t(
				"market.manage.selectAll",
			)}</button
		>
		<button
			type="button"
			disabled={busy || !market.checkedFiles.size}
			onclick={market.clearChecked}
			><Icon name="ui:close" size={15} />{t(
				"market.manage.clearSelection",
			)}</button
		>
		<span class="counts" class:has-selection={market.checkedFiles.size > 0}
			>{t("market.manage.counts", {
				total: market.localCount,
				filtered: market.total,
				selected: market.checkedFiles.size,
			})}</span
		>
	</div>
	{#if market.checkedFiles.size}
		<div
			class="batch-tools"
			role="group"
			aria-label={t("market.manage.selectedActions")}
		>
			{#if contentType === "mods"}
				<button
					type="button"
					disabled={busy}
					onclick={() => market.manageLocal("enable")}
					><Icon name="ui:check" size={15} />{t(
						"market.manage.enableSelected",
					)}</button
				>
				<button
					type="button"
					disabled={busy}
					onclick={() => market.manageLocal("disable")}
					><Icon name="ui:close" size={15} />{t(
						"market.manage.disableSelected",
					)}</button
				>
			{/if}
			<button
				type="button"
				class="danger"
				disabled={busy}
				onclick={() => onDelete([...market.checkedFiles])}
				><Icon name="ui:trash" size={15} />{t(
					"market.manage.deleteSelected",
				)}</button
			>
		</div>
	{/if}
	{#if market.instanceBusy}
		<p class="hint">
			<Icon name="instance:clock" size={15} />{t(
				"market.manage.instanceBusy",
			)}
		</p>
	{:else if contentType !== "mods"}
		<p class="hint">
			<Icon name="instance:box" size={15} />{t("market.manage.packHint")}
		</p>
	{/if}
	{#if report}
		<div
			class="operation-status"
			class:has-errors={!market.localOperationBusy &&
				report.failures.length > 0}
			role="status"
			aria-live="polite"
		>
			{#if market.localOperationBusy}
				<progress
					value={report.completed}
					max={report.total}
					aria-label={t("market.manage.progress", {
						completed: report.completed,
						total: report.total,
					})}
				></progress>
				{t("market.manage.progress", {
					completed: report.completed,
					total: report.total,
				})}
			{:else}
				<Icon
					name={report.failures.length
						? "ui:error"
						: "ui:check-circle"}
					size={16}
				/>
				{t("market.manage.result", {
					succeeded: report.succeeded,
					failed: report.failures.length,
				})}
			{/if}
		</div>
		{#if !market.localOperationBusy && report.failures.length}
			<details class="failure-report">
				<summary
					><span class="report-chevron"
						><Icon name="ui:chevron-right" size={15} /></span
					>{t("market.manage.showFailures", {
						count: report.failures.length,
					})}</summary
				>
				<ul>
					{#each report.failures.slice(0, 50) as failure (failure.filename)}<li
						>
							<strong>{failure.filename}</strong><span
								>{failure.error}</span
							>
						</li>{/each}
				</ul>
				<button type="button" onclick={copyFailures}
					><Icon name="ui:copy" size={15} />{t(
						"market.manage.copyFailures",
					)}</button
				>
				{#if report.failures.length > 50}<p>
						{t("market.manage.moreFailures", {
							count: report.failures.length - 50,
						})}
					</p>{/if}
			</details>
		{/if}
	{/if}
</div>

<style>
	.installed-toolbar {
		padding: 12px 24px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		border-bottom: 1px solid var(--border);
		background: var(--bg-main);
		color: var(--text-primary);
		font-size: 0.8rem;
		max-height: 40vh;
		overflow-y: auto;
		flex-shrink: 0;
	}
	.tools,
	.selection-tools,
	.batch-tools,
	.segmented-control,
	.operation-status {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}
	.view-options {
		margin-left: auto;
		flex-wrap: nowrap;
	}
	.segmented-control {
		gap: 2px;
		padding: 2px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--surface-subtle);
	}
	button {
		appearance: none;
		font: inherit;
		color: var(--text-primary);
		background: var(--surface-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 7px 10px;
		cursor: pointer;
		min-height: 34px;
		box-sizing: border-box;
		line-height: 1.2;
		transition:
			background-color 120ms ease,
			border-color 120ms ease,
			color 120ms ease;
	}
	button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		justify-content: center;
	}
	button:hover:not(:disabled) {
		border-color: var(--border-hover);
		background: var(--surface-hover);
	}
	button:active:not(:disabled) {
		background: var(--surface-active);
	}
	button.primary {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--accent-text);
		font-weight: 600;
	}
	button.primary:hover:not(:disabled) {
		background: var(--accent-hover);
		border-color: var(--accent-hover);
	}
	.segmented-control button {
		background: transparent;
		border-color: transparent;
		color: var(--text-secondary);
		min-height: 28px;
		padding: 5px 10px;
		font-size: 0.75rem;
		white-space: nowrap;
	}
	button[aria-pressed="true"] {
		border-color: var(--border-hover);
		background: var(--surface-selected);
		color: var(--text-primary);
		box-shadow: var(--shadow-sm);
	}
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	button:focus-visible,
	summary:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.status-filter {
		max-width: 100%;
		box-sizing: border-box;
	}
	.counts {
		color: var(--text-secondary);
		margin-left: auto;
		font-size: 0.75rem;
		line-height: 1.5;
		font-variant-numeric: tabular-nums;
		padding: 5px 8px;
		border-radius: var(--border-radius-sm);
	}
	.counts.has-selection {
		color: var(--text-primary);
		background: var(--surface-selected);
	}
	.batch-tools {
		padding: 8px;
		border: 1px solid var(--border-hover);
		border-radius: var(--border-radius);
		background: var(--surface-subtle);
	}
	.danger {
		color: var(--color-error);
		margin-left: auto;
	}
	.danger:hover:not(:disabled) {
		background: color-mix(
			in srgb,
			var(--color-error) 10%,
			var(--surface-card)
		);
		border-color: var(--color-error);
	}
	p {
		margin: 0;
		color: var(--text-secondary);
	}
	.hint {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 10px;
		border-radius: var(--border-radius-sm);
		background: var(--surface-subtle);
		font-size: 0.75rem;
		line-height: 1.5;
	}
	.operation-status {
		color: var(--text-secondary);
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--surface-card);
		font-variant-numeric: tabular-nums;
	}
	.operation-status.has-errors {
		border-color: color-mix(in srgb, var(--color-error) 35%, var(--border));
		color: var(--color-error);
	}
	progress {
		appearance: none;
		-webkit-appearance: none;
		border: 0;
		border-radius: 999px;
		overflow: hidden;
		background: var(--surface-active);
		height: 6px;
		width: 120px;
		accent-color: var(--accent);
	}
	progress::-webkit-progress-bar {
		background: var(--surface-active);
		border-radius: 999px;
	}
	progress::-webkit-progress-value {
		background: var(--accent);
		border-radius: 999px;
	}
	progress::-moz-progress-bar {
		background: var(--accent);
		border-radius: 999px;
	}
	.failure-report {
		color: var(--text-secondary);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--surface-card);
	}
	summary {
		cursor: pointer;
		list-style: none;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		border-radius: var(--border-radius);
		color: var(--text-primary);
	}
	summary::-webkit-details-marker {
		display: none;
	}
	summary:hover {
		background: var(--surface-hover);
	}
	.report-chevron {
		display: flex;
	}
	details[open] .report-chevron {
		transform: rotate(90deg);
	}
	.failure-report > button,
	.failure-report > p {
		margin: 0 12px 12px;
	}
	.failure-report > p {
		font-size: 0.75rem;
		line-height: 1.5;
	}
	li {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 8px 10px;
		font-size: 0.75rem;
		line-height: 1.5;
		border-bottom: 1px solid var(--border);
	}
	li:last-child {
		border-bottom: 0;
	}
	li strong {
		color: var(--text-primary);
		font-weight: 500;
	}
	li span {
		color: var(--text-secondary);
	}
	@media (prefers-reduced-motion: reduce) {
		button {
			transition: none;
		}
	}
	ul {
		max-height: 120px;
		overflow-y: auto;
		list-style: none;
		margin: 0 12px 12px;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
		overflow-wrap: anywhere;
	}
	@container market (max-width: 700px) {
		.installed-toolbar {
			padding: 10px 14px;
		}
	}
	@container market (max-width: 550px) {
		.view-options {
			margin-left: 0;
		}
		.counts {
			margin-left: 0;
			flex-basis: 100%;
		}
		.danger {
			margin-left: 0;
		}
		.tools > button {
			flex: 1 1 auto;
		}
	}
</style>
