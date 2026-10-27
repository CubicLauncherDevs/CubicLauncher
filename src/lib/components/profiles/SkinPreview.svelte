<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import Skin3dViewer from "./Skin3dViewer.svelte";
	import { launcherStore } from "$lib/state/state.svelte";

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
		draggingPng,
		dropTargetActive,
		onDragEnter,
		onDragLeave,
		onDragOver,
		onDrop,
	}: Props = $props();

	// Girar y caminar, a gusto de quien mira: se recuerda entre aperturas.
	const MOTION_KEY = "skin-preview-motion";
	function storedMotion(): { rotate: boolean; walk: boolean; eyes: boolean } {
		try {
			const kept = JSON.parse(localStorage.getItem(MOTION_KEY) ?? "{}");
			return {
				rotate: kept.rotate !== false,
				walk: kept.walk === true,
				eyes: kept.eyes !== false,
			};
		} catch {
			return { rotate: true, walk: false, eyes: true };
		}
	}
	const motion = $state(storedMotion());
	/** Si la skin lleva ojos de Fresh Moves: solo entonces hay botón de ojos. */
	let hasEyes = $state(false);
	function toggle(which: "rotate" | "walk" | "eyes") {
		motion[which] = !motion[which];
		try {
			localStorage.setItem(MOTION_KEY, JSON.stringify(motion));
		} catch {
			// Sin almacenamiento la elección dura lo que la ventana.
		}
	}
	// Con las animaciones 3D desactivadas en Ajustes no hay nada que encender.
	const motionAllowed = $derived(
		!launcherStore.settings.disable_skin3d_animations,
	);
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
		<Skin3dViewer
			{skinUrl}
			{capeUrl}
			{model}
			quality="high"
			rotate={motion.rotate}
			walk={motion.walk}
			eyes={motion.eyes}
			onEyesFound={(found) => (hasEyes = found)}
		/>

		{#if motionAllowed}
			<div class="motion-controls">
				<button
					type="button"
					class="motion-btn"
					class:active={motion.rotate}
					aria-pressed={motion.rotate}
					onclick={() => toggle("rotate")}
					title={t("userMenu.skinCape.rotate")}
					aria-label={t("userMenu.skinCape.rotate")}
				>
					<Icon name="ui:refresh" size={14} />
				</button>
				<button
					type="button"
					class="motion-btn"
					class:active={motion.walk}
					aria-pressed={motion.walk}
					onclick={() => toggle("walk")}
					title={t("userMenu.skinCape.walk")}
					aria-label={t("userMenu.skinCape.walk")}
				>
					<Icon name="ui:play" size={14} />
				</button>
				{#if hasEyes}
					<!-- En el juego los ojos solo se mueven con el paquete: lo dice la burbuja. -->
					<button
						type="button"
						class="motion-btn"
						class:active={motion.eyes}
						aria-pressed={motion.eyes}
						onclick={() => toggle("eyes")}
						title={t("userMenu.skinCape.eyesMove")}
						aria-label={t("userMenu.skinCape.eyesMove")}
					>
						<Icon name="ui:eye" size={14} />
					</button>
				{/if}
			</div>
		{/if}

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

	.motion-controls {
		position: absolute;
		top: 8px;
		left: 8px;
		z-index: 3;
		display: flex;
		gap: 6px;
	}

	.motion-btn {
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		background: var(--media-overlay);
		border: var(--border-width) solid var(--media-overlay-border);
		color: var(--media-overlay-text);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition:
			background 0.15s ease,
			color 0.15s ease;
	}

	.motion-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.motion-btn.active {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--accent-text);
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
