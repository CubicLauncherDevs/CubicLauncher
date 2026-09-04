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
		gap: 12px;
		padding: 10px 14px;
		background: var(--bg-card);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.header-main {
		display: flex;
		align-items: center;
		gap: 10px;
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
		gap: 2px;
		min-width: 0;
	}

	.log-title {
		font-size: 0.92rem;
		font-weight: 700;
		color: var(--text-primary);
		letter-spacing: -0.2px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.meta-row {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 0.65rem;
		color: var(--text-tertiary);
	}

	.meta-dot {
		width: 6px;
		height: 6px;
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
		padding: 1px 6px;
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-weight: 600;
	}

	.log-toolbar {
		display: flex;
		align-items: center;
		gap: 5px;
		flex-shrink: 0;
	}

	:global(.toolbar-icon) {
		width: 15px;
		height: 15px;
		flex-shrink: 0;
		filter: var(--icon-filter);
	}

	.toolbar-btn {
		height: 28px;
		width: 28px;
		padding: 0;
		border-radius: var(--border-radius-sm);
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-tertiary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s ease;
		font-size: 0.7rem;
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
		display: none;
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

	@media (max-width: 520px) {
		.log-header {
			gap: 10px;
		}

		.log-title {
			font-size: 0.85rem;
		}
	}
</style>
