<script lang="ts">
	import type { Snippet } from "svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import { t } from "$lib/i18n";
	import LogMenu from "./LogMenu.svelte";

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
</script>

<header class="log-header">
	<h1 title={instanceName}>{instanceName}</h1>
	<div class="controls">{@render children()}</div>
	<div class="actions">
		<LogMenu
			label={t(uploading ? "logWindow.uploading" : "logWindow.actions")}
		>
			{#snippet children({ close })}
				<button
					type="button"
					disabled={totalLines === 0}
					onclick={() => {
						close();
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
						close();
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
						close();
						onClear();
					}}
				>
					<Icon name="log:clear" size={14} />
					{t("logWindow.clear")}
				</button>
			{/snippet}
		</LogMenu>
	</div>
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
		flex-shrink: 0;
	}

	button {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		min-height: 32px;
		padding: 6px 8px;
		border: none;
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	button:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
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
