<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";

	interface Props {
		skinModel: "classic" | "slim";
		showUrl: boolean;
		skinUrlInput: string;
		processing: boolean;
		onModelChange: (model: "classic" | "slim") => void;
		onFileSelect: () => void;
		onUrlToggle: () => void;
		onUrlSubmit: () => void;
	}

	let {
		skinModel,
		showUrl,
		skinUrlInput = $bindable(),
		processing,
		onModelChange,
		onFileSelect,
		onUrlToggle,
		onUrlSubmit,
	}: Props = $props();
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
			<button
				type="button"
				class="btn-secondary url-toggle"
				onclick={onUrlToggle}
				disabled={processing}
				aria-expanded={showUrl}
			>
				{t("userMenu.skinCape.useUrl")}
			</button>
		</div>
	</div>

	{#if showUrl}
		<div class="url-row">
			<input
				type="text"
				bind:value={skinUrlInput}
				placeholder={t("userMenu.skinCape.skinUrlPlaceholder")}
				class="url-input"
				onkeydown={(e) => e.key === "Enter" && onUrlSubmit()}
				disabled={processing}
			/>
			<button
				type="button"
				class="btn-primary url-submit"
				onclick={onUrlSubmit}
				disabled={processing || !skinUrlInput.trim()}
			>
				{t("userMenu.skinCape.useUrl")}
			</button>
		</div>
	{/if}
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

	.upload-btn,
	.url-toggle,
	.url-submit {
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
	}

	.upload-btn {
		background: var(--accent);
		color: var(--accent-text);
		border: 1px solid var(--accent);
	}

	.upload-btn:hover:not(:disabled) {
		background: var(--accent-hover);
	}

	.url-toggle,
	.url-submit {
		background: transparent;
		color: var(--text-secondary);
		border: 1px solid var(--border);
	}

	.url-toggle:hover:not(:disabled),
	.url-submit:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.upload-btn:disabled,
	.url-toggle:disabled,
	.url-submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.url-row {
		display: flex;
		gap: 6px;
		align-items: center;
		animation: slideDown 0.15s ease;
	}

	@keyframes slideDown {
		from {
			opacity: 0;
			transform: translateY(-4px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.url-input {
		flex: 1;
		min-width: 0;
		background: var(--bg-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		padding: 6px 8px;
		font-family: inherit;
		font-size: 0.75rem;
		outline: none;
	}

	.url-input:focus {
		border-color: var(--text-muted);
	}

	@media (max-width: 420px) {
		.controls-row {
			flex-direction: column;
			align-items: stretch;
		}

		.skin-actions {
			width: 100%;
		}

		.upload-btn,
		.url-toggle {
			flex: 1;
		}
	}
</style>
