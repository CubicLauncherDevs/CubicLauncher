<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import Skin3dViewer from "./Skin3dViewer.svelte";

	interface Props {
		skinUrl: string;
		capeUrl: string | null;
		model: "default" | "slim";
		variant: string | undefined;
		draggingPng: boolean;
		dropTargetActive: boolean;
		onDragEnter: (e: DragEvent) => void;
		onDragLeave: (e: DragEvent) => void;
		onDragOver: (e: DragEvent) => void;
		onDrop: (e: DragEvent) => void;
	}

	let {
		skinUrl,
		capeUrl,
		model,
		variant,
		draggingPng,
		dropTargetActive,
		onDragEnter,
		onDragLeave,
		onDragOver,
		onDrop,
	}: Props = $props();
</script>

<div
	class="preview-zone"
	class:drop-ready={draggingPng && dropTargetActive}
	role="button"
	tabindex="0"
	ondragenter={onDragEnter}
	ondragleave={onDragLeave}
	ondragover={onDragOver}
	ondrop={onDrop}
>
	{#if skinUrl}
		<Skin3dViewer {skinUrl} {capeUrl} {model} />

		{#if draggingPng && dropTargetActive}
			<div class="drop-overlay">
				<Icon name="ui:upload" size={32} />
				<span>{t("userMenu.skinCape.dropSkinHere")}</span>
			</div>
		{/if}
	{:else}
		<div class="empty-preview">
			<span>No hay skin activa</span>
		</div>
	{/if}
</div>

<style>
	.preview-zone {
		position: relative;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		overflow: hidden;
		min-height: 260px;
		height: 300px;
		transition:
			border-color 0.15s ease,
			background 0.15s ease;
		outline: none;
	}

	.preview-zone:focus-visible {
		border-color: var(--accent);
	}

	.preview-zone.drop-ready {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.04);
	}

	.drop-overlay {
		position: absolute;
		inset: 0;
		z-index: 4;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		background: var(--bg-overlay);
		backdrop-filter: blur(var(--backdrop-blur-modal));
		color: var(--text-primary);
		font-size: 0.85rem;
		font-weight: 600;
	}

	.empty-preview {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		font-size: 0.85rem;
	}
</style>
