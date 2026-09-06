<script lang="ts">
	import type { Snippet } from "svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import { t } from "$lib/i18n";

	interface Props {
		instanceName: string;
		totalLines: number;
		uploading: boolean;
		onClear: () => void;
		onCopy: () => void;
		onUpload: () => void;
		children: Snippet;
	}

	let {
		instanceName,
		totalLines,
		uploading,
		onClear,
		onCopy,
		onUpload,
		children,
	}: Props = $props();
	let actions: HTMLDetailsElement;
	let trigger: HTMLElement;

	function closeActions() {
		if (!actions?.open) return;
		actions.open = false;
		trigger.focus();
	}
</script>

<svelte:window
	onclick={(event) => {
		if (actions && !actions.contains(event.target as Node))
			actions.open = false;
	}}
	onkeydown={(event) => {
		if (event.key === "Escape") closeActions();
	}}
/>

<header class="log-header">
	<h1 title={instanceName}>{instanceName}</h1>
	<div class="controls">{@render children()}</div>
	<details class="actions" bind:this={actions}>
		<summary bind:this={trigger}>
			{t(uploading ? "logWindow.uploading" : "logWindow.actions")}
			<Icon name="log:chevron-down" size={12} />
		</summary>
		<div class="action-list">
			<button
				type="button"
				disabled={totalLines === 0}
				onclick={() => {
					closeActions();
					onCopy();
				}}
			>
				<Icon name="log:copy" size={14} />
				{t("logWindow.copy")}
			</button>
			<button
				type="button"
				disabled={totalLines === 0 || uploading}
				onclick={() => {
					closeActions();
					onUpload();
				}}
			>
				<Icon name="log:upload" size={14} />
				{t(uploading ? "logWindow.uploading" : "logWindow.upload")}
			</button>
			<button
				type="button"
				class="clear-action"
				disabled={totalLines === 0}
				onclick={() => {
					closeActions();
					onClear();
				}}
			>
				<Icon name="log:clear" size={14} />
				{t("logWindow.clear")}
			</button>
		</div>
	</details>
</header>

<style>
	.log-header {
		display: grid;
		grid-template-columns: minmax(0, 160px) minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		padding: 8px 14px;
		background: var(--bg-card);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.controls {
		min-width: 0;
	}

	h1 {
		font-size: 0.82rem;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.actions {
		position: relative;
		flex-shrink: 0;
	}

	summary,
	button {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 32px;
		padding: 6px 8px;
		border: none;
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}

	summary {
		list-style: none;
	}
	summary::-webkit-details-marker {
		display: none;
	}

	summary:hover,
	.actions[open] summary,
	button:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	summary:focus-visible,
	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.action-list {
		position: absolute;
		top: calc(100% + 4px);
		right: 0;
		z-index: 20;
		width: max-content;
		max-width: calc(100vw - 28px);
		padding: 4px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		box-shadow: var(--shadow-md);
	}

	button {
		width: 100%;
		text-align: left;
	}
	button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.clear-action {
		border-top: 1px solid var(--border);
		border-radius: 0;
		margin-top: 4px;
	}

	@media (max-width: 700px) {
		.log-header {
			grid-template-columns: minmax(0, 1fr) auto;
			gap: 6px 12px;
		}
		.controls {
			grid-column: 1 / -1;
			grid-row: 2;
		}
		.actions {
			grid-column: 2;
			grid-row: 1;
		}
	}
</style>
