<script lang="ts">
	import { onMount } from "svelte";
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import Skin3dViewer from "./Skin3dViewer.svelte";
	import type { SkinClosetEntry } from "$lib/types/types";

	interface Props {
		entry: SkinClosetEntry;
		isActive: boolean;
		processing: boolean;
		onEquip: () => void;
		onRemove: () => void;
		onRename: (alias: string) => void;
	}

	let { entry, isActive, processing, onEquip, onRemove, onRename }: Props =
		$props();

	let editing = $state(false);
	let aliasInput = $state("");
	let visible = $state(false);
	let previewContainer: HTMLElement;

	onMount(() => {
		const observer = new IntersectionObserver(
			(observed) => {
				if (observed[0]?.isIntersecting) {
					visible = true;
				}
			},
			{ rootMargin: "80px", threshold: 0 },
		);

		if (previewContainer) {
			observer.observe(previewContainer);
		}

		return () => observer.disconnect();
	});

	$effect(() => {
		if (!editing) {
			aliasInput = entry.alias;
		}
	});

	function startEditing(event: MouseEvent) {
		event.stopPropagation();
		aliasInput = entry.alias;
		editing = true;
	}

	function cancelEditing() {
		editing = false;
		aliasInput = entry.alias;
	}

	function saveAlias() {
		editing = false;
		if (aliasInput.trim() !== entry.alias) {
			onRename(aliasInput.trim());
		}
	}

	function handleRemove(event: MouseEvent) {
		event.stopPropagation();
		onRemove();
	}

	const displayName = $derived(entry.alias || entry.id.slice(0, 8));
	const variantLabel = $derived(
		entry.variant.toUpperCase() === "SLIM"
			? t("userMenu.skinCape.slim")
			: t("userMenu.skinCape.classic"),
	);
	const viewerModel = $derived(
		entry.variant.toUpperCase() === "SLIM" ? "slim" : "default",
	);
</script>

<div class="closet-card" class:active={isActive}>
	<div class="card-preview" bind:this={previewContainer}>
		{#if visible}
			<Skin3dViewer
				skinUrl={entry.url}
				model={viewerModel}
				animated={false}
			/>
		{:else}
			<div class="preview-placeholder"></div>
		{/if}

		{#if !editing}
			<div class="card-actions">
				<button
					type="button"
					class="icon-btn"
					onclick={startEditing}
					disabled={processing}
					aria-label={t("userMenu.skinCape.skinCloset.renameSkin")}
					title={t("userMenu.skinCape.skinCloset.renameSkin")}
				>
					<Icon name="nav:pencil" size={12} />
				</button>
				<button
					type="button"
					class="icon-btn danger"
					onclick={handleRemove}
					disabled={processing}
					aria-label={t("userMenu.skinCape.skinCloset.deleteSkin")}
					title={t("userMenu.skinCape.skinCloset.deleteSkin")}
				>
					<Icon name="ui:trash" size={12} />
				</button>
			</div>

		{/if}

		{#if isActive}
			<span
				class="active-badge"
				aria-label={t("userMenu.skinCape.active")}
			>
				<Icon name="ui:check" size={10} />
			</span>
		{/if}

		<span class="variant-badge">{variantLabel}</span>
	</div>

	<div class="card-info">
		{#if editing}
			<input
				type="text"
				class="alias-input"
				placeholder={t(
					"userMenu.skinCape.skinCloset.skinNamePlaceholder",
				)}
				bind:value={aliasInput}
				disabled={processing}
			/>
			<div class="alias-actions">
				<button
					type="button"
					class="btn-secondary alias-btn"
					onclick={cancelEditing}
					disabled={processing}
				>
					{t("userMenu.skinCape.skinCloset.cancelAlias")}
				</button>
				<button
					type="button"
					class="btn-primary alias-btn"
					onclick={saveAlias}
					disabled={processing}
				>
					{t("userMenu.skinCape.skinCloset.saveAlias")}
				</button>
			</div>
		{:else}
			<div class="card-name-row">
				<span class="card-name" title={displayName}>{displayName}</span>
				{#if !isActive}
					<button
						type="button"
						class="equip-btn"
						onclick={onEquip}
						disabled={processing}
					>
						{t("userMenu.skinCape.skinCloset.equipSkin")}
					</button>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.closet-card {
		display: flex;
		flex-direction: column;
		background: var(--bg-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		overflow: hidden;
		transition:
			transform 0.15s ease,
			background 0.15s ease,
			border-color 0.15s ease,
			box-shadow 0.15s ease;
	}

	.closet-card:hover {
		transform: translateY(-2px);
		background: var(--surface-selected);
		box-shadow: 0 4px 14px rgba(0, 0, 0, 0.2);
	}

	.closet-card.active {
		background: var(--surface-selected);
		box-shadow:
			0 4px 14px rgba(0, 0, 0, 0.2),
			inset 0 2px 0 0 var(--accent);
	}

	.closet-card.active .card-preview {
		box-shadow: inset 0 2px 0 0 var(--accent);
	}

	.card-preview {
		width: 100%;
		height: 140px;
		position: relative;
		background: var(--bg-card);
		border-bottom: 1px solid var(--border);
		overflow: hidden;
	}

	.card-preview :global(.skin-3d-viewer) {
		min-width: auto;
		min-height: auto;
		width: 100%;
		height: 100%;
	}

	.preview-placeholder {
		width: 100%;
		height: 100%;
		background: var(--bg-card);
	}

	.card-actions {
		position: absolute;
		top: 6px;
		right: 6px;
		display: flex;
		gap: 4px;
		opacity: 0;
		transition: opacity 0.15s ease;
	}

	.closet-card:hover .card-actions,
	.closet-card:focus-within .card-actions {
		opacity: 1;
	}

	@media (hover: none) {
		.card-actions {
			opacity: 1;
		}
	}

	.icon-btn {
		background: rgba(0, 0, 0, 0.5);
		backdrop-filter: blur(4px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: #fff;
		border-radius: var(--border-radius-sm);
		width: 22px;
		height: 22px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition:
			background 0.15s ease,
			color 0.15s ease;
	}

	.icon-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.icon-btn.danger:hover:not(:disabled) {
		color: var(--error, #ff6b6b);
		border-color: var(--error, #ff6b6b);
	}

	.icon-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.active-badge {
		position: absolute;
		top: 6px;
		left: 6px;
		width: 20px;
		height: 20px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--accent);
		color: var(--accent-text);
		border-radius: 50%;
	}

	.variant-badge {
		position: absolute;
		bottom: 6px;
		left: 6px;
		background: rgba(0, 0, 0, 0.6);
		color: #fff;
		font-size: 0.55rem;
		font-weight: 700;
		padding: 1px 5px;
		border-radius: 4px;
		text-transform: uppercase;
	}

	.card-info {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		padding: 10px 8px;
		text-align: center;
		min-height: 52px;
	}

	.card-name-row {
		position: relative;
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 1.2em;
	}

	.card-name {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		width: 100%;
		transition: opacity 0.15s ease;
	}

	.alias-input {
		width: 100%;
		font-family: inherit;
		font-size: 0.75rem;
		padding: 4px 6px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-card);
		color: var(--text-primary);
		text-align: center;
	}

	.alias-actions {
		display: flex;
		gap: 4px;
		margin-top: 4px;
	}

	.alias-btn {
		font-family: inherit;
		font-size: 0.6rem;
		font-weight: 600;
		padding: 3px 6px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		border: 1px solid transparent;
	}

	.equip-btn {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		font-family: inherit;
		font-size: 0.75rem;
		font-weight: 700;
		padding: 2px 10px;
		background: var(--accent);
		color: var(--accent-text);
		border: none;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		opacity: 0;
		pointer-events: none;
		transition:
			opacity 0.15s ease,
			background 0.15s ease;
	}

	.closet-card:hover .equip-btn,
	.closet-card:focus-within .equip-btn,
	.card-name-row:focus-within .equip-btn {
		opacity: 1;
		pointer-events: auto;
	}

	.closet-card:hover .card-name,
	.closet-card:focus-within .card-name,
	.card-name-row:focus-within .card-name {
		opacity: 0;
	}

	@media (hover: none) {
		.equip-btn {
			opacity: 1;
			pointer-events: auto;
			position: relative;
			inset: auto;
			padding: 4px 10px;
		}

		.closet-card:hover .card-name,
		.closet-card:focus-within .card-name,
		.card-name-row:focus-within .card-name {
			opacity: 1;
		}
	}

	.equip-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.equip-btn:hover:not(:disabled) {
		background: var(--accent-hover);
	}

	@media (max-width: 520px) {
		.card-preview {
			height: 110px;
		}

		.card-info {
			padding: 8px 6px;
			min-height: 46px;
		}

		.card-name {
			font-size: 0.75rem;
		}
	}
</style>
