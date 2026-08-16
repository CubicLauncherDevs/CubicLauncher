<script lang="ts">
	import { t } from "$lib/i18n";

	interface Props {
		processing: boolean;
		onSave: () => void;
		onDiscard: () => void;
	}

	let { processing, onSave, onDiscard }: Props = $props();
</script>

<div class="pending-bar">
	<span class="pending-info">
		<span class="pending-dot"></span>
		{t("userMenu.skinCape.pendingChanges")}
	</span>
	<div class="pending-actions">
		<button
			type="button"
			class="btn-secondary discard-btn"
			onclick={onDiscard}
			disabled={processing}
		>
			{t("userMenu.skinCape.discardChanges")}
		</button>
		<button
			type="button"
			class="btn-primary save-btn"
			onclick={onSave}
			disabled={processing}
		>
			{#if processing}
				<span class="spinner"></span>
			{/if}
			{t("userMenu.skinCape.saveChanges")}
		</button>
	</div>
</div>

<style>
	.pending-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
		background: rgba(var(--accent-rgb), 0.08);
		border: 1px solid var(--accent);
		border-radius: var(--border-radius);
		padding: 10px 12px;
	}

	.pending-info {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.pending-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--accent);
		animation: pulse 1.2s ease-in-out infinite;
		flex-shrink: 0;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.35;
		}
	}

	.pending-actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.pending-actions .spinner {
		width: 12px;
		height: 12px;
		margin-right: 6px;
	}

	.save-btn,
	.discard-btn {
		display: inline-flex;
		align-items: center;
		font-size: 0.78rem;
		padding: 7px 14px;
	}

	.btn-primary,
	.btn-secondary {
		font-family: inherit;
		font-weight: 600;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition: all 0.15s ease;
		border: 1px solid transparent;
		white-space: nowrap;
	}

	.btn-primary {
		background: var(--accent);
		color: var(--accent-text);
	}

	.btn-primary:hover:not(:disabled) {
		opacity: 0.85;
	}

	.btn-secondary {
		background: transparent;
		border-color: var(--border);
		color: var(--text-secondary);
	}

	.btn-secondary:hover:not(:disabled) {
		background: var(--surface-selected);
		color: var(--text-primary);
	}

	.btn-primary:disabled,
	.btn-secondary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.spinner {
		width: 16px;
		height: 16px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
