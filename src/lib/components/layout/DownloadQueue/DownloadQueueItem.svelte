<script lang="ts">
	import { t } from "$lib/i18n";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";

	let {
		version,
		overall,
		done,
		error,
		statusLabel,
	}: {
		version: string;
		overall: number;
		done: boolean;
		error: string | null;
		statusLabel: string | null;
	} = $props();
</script>

<div class="sd-item" class:done class:error>
	<div class="sd-item-header">
		<span class="sd-item-left">
			{#if error}
				<span class="sd-error-icon">!</span>
			{:else if done}
				<CheckIcon size={8} />
			{:else}
				<span class="sd-spinner-sm"></span>
			{/if}
			<div class="sd-version-wrap">
				<span class="sd-version"
					>{version === "mods"
						? t("sidebar.downloadingMods")
						: version}</span
				>
				{#if statusLabel}
					<span class="sd-status-label">{statusLabel}</span>
				{/if}
			</div>
		</span>
		{#if error}
			<span class="sd-pct error">{t("sidebar.failed")}</span>
		{:else}
			<span class="sd-pct" class:done={done}>{overall}%</span>
		{/if}
	</div>
	{#if error}
		<div class="sd-error-msg">{error}</div>
	{:else}
		<div class="sd-progress-track">
			<div
				class="sd-progress-fill"
				class:done={done}
				style:width="{overall}%"
			></div>
		</div>
	{/if}
</div>

<style>
	.sd-item {
		padding: 8px 10px;
		border-bottom: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.sd-item:last-child {
		border-bottom: none;
	}

	.sd-item-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.sd-item-left {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		flex: 1;
	}

	.sd-version-wrap {
		display: flex;
		flex-direction: column;
		gap: 1px;
		min-width: 0;
	}

	.sd-version {
		font-size: 0.72rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.sd-status-label {
		font-size: 0.6rem;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.sd-pct {
		font-size: 0.65rem;
		font-weight: 700;
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.sd-pct.done {
		color: var(--color-success);
	}

	.sd-pct.error {
		color: var(--color-error);
	}

	.sd-error-icon {
		width: 8px;
		height: 8px;
		background: var(--color-error);
		color: white;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 6px;
		font-weight: 900;
		line-height: 1;
		flex-shrink: 0;
	}

	.sd-error-msg {
		font-size: 0.65rem;
		color: var(--color-error);
		word-break: break-word;
		line-height: 1.3;
	}

	.sd-item.error {
		background: rgba(220, 38, 38, 0.05);
	}

	.sd-progress-track {
		width: 100%;
		height: 3px;
		background: var(--bg-input);
		border-radius: 2px;
		overflow: hidden;
	}

	.sd-progress-fill {
		height: 100%;
		background: var(--accent);
		border-radius: 2px;
		transition: width 0.35s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.sd-progress-fill.done {
		background: var(--color-success);
	}

	.sd-spinner-sm {
		width: 8px;
		height: 8px;
		border: 1.5px solid var(--border);
		border-top-color: var(--text-muted);
		border-radius: 50%;
		animation: sd-spin 0.7s linear infinite;
		will-change: transform;
		flex-shrink: 0;
		display: block;
	}

	@keyframes sd-spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 650px) {
		.sd-item {
			padding: 6px 4px;
			align-items: center;
		}

		.sd-item-header {
			justify-content: center;
		}

		.sd-item .sd-spinner-sm {
			width: 10px;
			height: 10px;
		}
	}
</style>