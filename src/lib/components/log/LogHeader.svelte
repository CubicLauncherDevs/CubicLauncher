<script lang="ts">
	import Icon from "$lib/icons/Icon.svelte";

	interface Props {
		instanceName: string;
		totalLines: number;
		isAtBottom: boolean;
		uploading: boolean;
		onClear: () => void;
		onCopy: () => void;
		onUpload: () => void;
		onScrollBottom: () => void;
	}

	let {
		instanceName,
		totalLines,
		isAtBottom,
		uploading,
		onClear,
		onCopy,
		onUpload,
		onScrollBottom,
	}: Props = $props();

	const formattedLines = $derived(totalLines.toLocaleString());
	const isLive = $derived(totalLines > 0);
	const uploadLabel = $derived(
		uploading ? "Subiendo log..." : "Subir a mclo.gs",
	);
</script>

<header class="log-header">
	<div class="header-main">
		<Icon
			name="log:logs"
			class="log-icon"
			size={38}
			style="filter: var(--icon-filter);"
		/>
		<div class="title-area">
			<h1 class="log-title">{instanceName}</h1>
			<div class="meta-row">
				<span class="meta-dot" class:alive={isLive}></span>
				<span class="meta-status">
					{isLive ? "En vivo" : "Esperando logs"}
				</span>
				<span class="meta-sep">·</span>
				<span class="meta-count">{formattedLines} líneas</span>
			</div>
		</div>
	</div>
	<div class="log-toolbar">
		<button
			type="button"
			class="toolbar-btn"
			onclick={onClear}
			disabled={totalLines === 0}
			title="Limpiar log"
			aria-label="Limpiar log"
		>
			<Icon name="log:clear" class="toolbar-icon" size={15} />
			<span class="btn-label">Limpiar</span>
		</button>
		<button
			type="button"
			class="toolbar-btn"
			onclick={onCopy}
			disabled={totalLines === 0}
			title="Copiar log"
			aria-label="Copiar log"
		>
			<Icon name="log:copy" class="toolbar-icon" size={15} />
			<span class="btn-label">Copiar</span>
		</button>
		<button
			type="button"
			class="toolbar-btn"
			onclick={onUpload}
			disabled={totalLines === 0 || uploading}
			title={uploadLabel}
			aria-label={uploadLabel}
		>
			{#if uploading}
				<Icon name="log:spinner" class="toolbar-icon spin" size={15} />
			{:else}
				<Icon name="log:upload" class="toolbar-icon" size={15} />
			{/if}
			<span class="btn-label">Subir</span>
		</button>
		<button
			type="button"
			class="toolbar-btn"
			class:active={isAtBottom}
			onclick={onScrollBottom}
			title="Ir al final"
			aria-label="Ir al final"
			aria-pressed={isAtBottom}
		>
			<Icon name="log:scroll-down" class="toolbar-icon" size={15} />
			<span class="btn-label">Auto-scroll</span>
		</button>
	</div>
</header>

<style>
	.log-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 18px 20px;
		background: var(--bg-card-gradient), var(--bg-card);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.header-main {
		display: flex;
		align-items: center;
		gap: 14px;
		min-width: 0;
	}

	:global(.log-icon) {
		width: 38px;
		height: 38px;
		border-radius: var(--border-radius-sm);
		flex-shrink: 0;
		opacity: 0.85;
	}

	.title-area {
		display: flex;
		flex-direction: column;
		gap: 5px;
		min-width: 0;
	}

	.log-title {
		font-size: 1.05rem;
		font-weight: 800;
		color: var(--text-primary);
		letter-spacing: -0.3px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.meta-row {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.72rem;
		color: var(--text-tertiary);
	}

	.meta-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--text-muted);
		transition: all 0.3s ease;
		flex-shrink: 0;
	}

	.meta-dot.alive {
		background: var(--color-status-started);
		box-shadow: var(--glow-success);
		animation: pulse 1.5s ease-in-out infinite;
	}

	.meta-status {
		font-weight: 600;
		white-space: nowrap;
	}

	.meta-sep {
		opacity: 0.6;
	}

	.meta-count {
		background: var(--surface-active);
		padding: 1px 7px;
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-weight: 600;
	}

	.log-toolbar {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
	}

	:global(.toolbar-icon) {
		width: 15px;
		height: 15px;
		flex-shrink: 0;
		filter: var(--icon-filter);
	}

	.toolbar-btn {
		height: 34px;
		padding: 0 8px;
		border-radius: var(--border-radius-sm);
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-tertiary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 5px;
		transition: all 0.15s ease;
		font-size: 0.75rem;
		font-weight: 600;
	}

	.toolbar-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		border-color: var(--text-tertiary);
		color: var(--text-primary);
	}

	.toolbar-btn.active {
		color: var(--color-status-started);
		background: color-mix(
			in srgb,
			var(--color-status-started) 12%,
			transparent
		);
		border-color: color-mix(
			in srgb,
			var(--color-status-started) 30%,
			transparent
		);
	}

	.toolbar-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.btn-label {
		white-space: nowrap;
		max-width: 0;
		min-width: 0;
		opacity: 0;
		overflow: hidden;
		transition:
			max-width 0.35s cubic-bezier(0.4, 0, 0.2, 1),
			opacity 0.25s ease,
			margin 0.25s ease;
		pointer-events: none;
	}

	.toolbar-btn:hover:not(:disabled) .btn-label {
		max-width: 90px;
		opacity: 1;
		margin-right: 1px;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}

	:global(.spin) {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 600px) {
		.log-header {
			flex-direction: column;
			align-items: flex-start;
			gap: 12px;
		}

		.log-toolbar {
			width: 100%;
			justify-content: flex-end;
		}
	}
</style>
