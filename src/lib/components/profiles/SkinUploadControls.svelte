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

<div class="model-selector">
	<button
		type="button"
		class="model-btn"
		class:active={skinModel === "classic"}
		onclick={() => onModelChange("classic")}
		disabled={processing}
	>
		<span class="model-label">{t("userMenu.skinCape.classic")}</span>
		{#if skinModel === "classic"}
			<Icon src="/images/icons/ui/check.svg" size={14} />
		{/if}
	</button>
	<button
		type="button"
		class="model-btn"
		class:active={skinModel === "slim"}
		onclick={() => onModelChange("slim")}
		disabled={processing}
	>
		<span class="model-label">{t("userMenu.skinCape.slim")}</span>
		{#if skinModel === "slim"}
			<Icon src="/images/icons/ui/check.svg" size={14} />
		{/if}
	</button>
</div>

<div class="skin-actions">
	<button
		type="button"
		class="btn-primary upload-btn"
		onclick={onFileSelect}
		disabled={processing}
	>
		<Icon src="/images/icons/ui/upload.svg" size={16} />
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
			class="btn-primary"
			onclick={onUrlSubmit}
			disabled={processing || !skinUrlInput.trim()}
		>
			{t("userMenu.skinCape.useUrl")}
		</button>
	</div>
{/if}

<style>
	.model-selector {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}

	.model-btn {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 10px 12px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s ease;
	}

	.model-btn:hover:not(:disabled, .active) {
		background: var(--surface-selected);
		color: var(--text-primary);
	}

	.model-btn.active {
		background: rgba(var(--accent-rgb), 0.12);
		border-color: rgba(var(--accent-rgb), 0.45);
		color: var(--accent);
	}

	.model-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.model-label {
		text-transform: capitalize;
	}

	.skin-actions {
		display: flex;
		gap: 10px;
	}

	.upload-btn {
		flex: 1;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
	}

	.url-toggle {
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}

	.url-row {
		display: flex;
		gap: 8px;
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
		padding: 8px 10px;
		font-family: inherit;
		font-size: 0.8rem;
		outline: none;
	}

	.url-input:focus {
		border-color: var(--text-muted);
	}
</style>
