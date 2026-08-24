<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";

	interface Props {
		skinModel: "classic" | "slim";
		processing: boolean;
		onModelChange: (model: "classic" | "slim") => void;
		onFileSelect: () => void;
	}

	let { skinModel, processing, onModelChange, onFileSelect }: Props =
		$props();
</script>

<div class="controls-stack">
	<div class="controls-row">
		<div class="model-selector">
			<button
				type="button"
				class="model-btn"
				class:active={skinModel === "classic"}
				onclick={() => onModelChange("classic")}
				disabled={processing}
			>
				{t("userMenu.skinCape.classic")}
			</button>
			<button
				type="button"
				class="model-btn"
				class:active={skinModel === "slim"}
				onclick={() => onModelChange("slim")}
				disabled={processing}
			>
				{t("userMenu.skinCape.slim")}
			</button>
		</div>

		<div class="skin-actions">
			<button
				type="button"
				class="btn-primary upload-btn"
				onclick={onFileSelect}
				disabled={processing}
			>
				<Icon src="/images/icons/ui/upload.svg" size={14} />
				<span>{t("userMenu.skinCape.uploadSkin")}</span>
			</button>
		</div>
	</div>
</div>

<style>
	.controls-stack {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.controls-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		flex-wrap: wrap;
	}

	.model-selector {
		display: inline-flex;
		gap: 4px;
	}

	.model-btn {
		background: var(--bg-input);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 5px 10px;
		border-radius: var(--border-radius-sm);
		font-family: inherit;
		font-size: 0.7rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.15s ease,
			border-color 0.15s ease,
			color 0.15s ease;
	}

	.model-btn:hover:not(:disabled, .active) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.model-btn.active {
		background: rgba(var(--accent-rgb), 0.12);
		border-color: var(--accent);
		color: var(--accent);
	}

	.model-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.skin-actions {
		display: inline-flex;
		gap: 6px;
	}

	.upload-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 5px 10px;
		border-radius: var(--border-radius-sm);
		font-family: inherit;
		font-size: 0.7rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.15s ease,
			border-color 0.15s ease,
			color 0.15s ease;
		background: var(--accent);
		color: var(--accent-text);
		border: 1px solid var(--accent);
	}

	.upload-btn:hover:not(:disabled) {
		background: var(--accent-hover);
	}

	.upload-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	@media (max-width: 420px) {
		.controls-row {
			flex-direction: column;
			align-items: stretch;
		}

		.skin-actions {
			width: 100%;
		}

		.upload-btn {
			width: 100%;
		}
	}
</style>
