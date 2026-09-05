<script lang="ts">
	import { onMount } from "svelte";
	import { fly } from "svelte/transition";
	import { t } from "$lib/i18n";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";
	import {
		getMinecraftProfile,
		getSkinPreviewData,
		uploadSkinFile,
		uploadSkinUrl,
		equipCape,
		unequipCape,
	} from "$lib/api/cubicApi";
	import { showError, showSuccess } from "$lib/state/state.svelte";
	import { bumpAvatarVersion } from "$lib/state/avatarCache.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import SkinPreview from "./SkinPreview.svelte";
	import CapeList from "./CapeList.svelte";
	import type {
		MinecraftProfileCape,
		MinecraftProfileResponse,
		MinecraftProfileSkin,
	} from "$lib/types/types";

	interface Props {
		uuid: string;
	}

	let { uuid }: Props = $props();

	let profile = $state<MinecraftProfileResponse | null>(null);
	let loading = $state(false);
	// Flag no reactivo para evitar rerenders/reintentos infinitos en Svelte 5.
	let fetchingProfile = false;
	let skinModel = $state<"classic" | "slim">("classic");
	let processing = $state(false);
	let draggingPng = $state(false);
	let dropTargetActive = $state(false);
	let originalCapeId = $state<string | null>(null);
	let selectedCapeId = $state<string | null>(null);
	let originalVariant = $state<"classic" | "slim">("classic");

	interface PendingSkin {
		filePath: string;
		variant: "CLASSIC" | "SLIM";
		previewUrl: string;
	}

	let pendingSkin = $state<PendingSkin | null>(null);

	async function loadProfile(silent = false) {
		if (fetchingProfile) return;
		fetchingProfile = true;
		if (!silent) loading = true;
		try {
			const p = await getMinecraftProfile(uuid);
			if (p) {
				profile = p;
				const activeSkin = p.skins.find((s) => s.state === "ACTIVE");
				if (activeSkin?.variant === "SLIM") {
					skinModel = "slim";
					originalVariant = "slim";
				} else {
					skinModel = "classic";
					originalVariant = "classic";
				}
				const activeCape = p.capes.find((c) => c.state === "ACTIVE");
				originalCapeId = activeCape?.id ?? null;
				selectedCapeId = originalCapeId;
				pendingSkin = null;
			}
		} finally {
			fetchingProfile = false;
			if (!silent) loading = false;
		}
	}

	function discardChanges() {
		pendingSkin = null;
		selectedCapeId = originalCapeId;
		skinModel = originalVariant;
	}

	async function handleFileUpload(filePath?: string) {
		let selected = filePath;
		if (!selected) {
			const dialogResult = await openDialog({
				multiple: false,
				filters: [
					{
						name: "Minecraft Skin",
						extensions: ["png"],
					},
				],
			});
			if (!dialogResult || Array.isArray(dialogResult)) return;
			selected = dialogResult;
		}
		let previewUrl: string;
		try {
			previewUrl = await getSkinPreviewData(selected);
		} catch (e) {
			showError(t("errors.title"), String(e));
			return;
		}
		pendingSkin = {
			filePath: selected,
			variant: skinModel === "slim" ? "SLIM" : "CLASSIC",
			previewUrl,
		};
	}

	async function handleDroppedSkinFile(paths: string[]) {
		const png = paths.find((p) => p.toLowerCase().endsWith(".png"));
		if (!png) {
			showError(
				t("errors.title"),
				t("userMenu.skinCape.invalidSkinFile"),
			);
			return;
		}
		await handleFileUpload(png);
	}

	function handleModelChange(model: "classic" | "slim") {
		skinModel = model;
		if (pendingSkin) {
			pendingSkin.variant = model === "slim" ? "SLIM" : "CLASSIC";
		}
	}

	function handleSelectCape(capeId: string | null) {
		selectedCapeId = capeId;
	}

	function isRateLimitError(err: unknown): boolean {
		const msg = String(err).toLowerCase();
		return (
			msg.includes("429") ||
			msg.includes("too many requests") ||
			msg.includes("rate limit")
		);
	}

	async function handleSaveChanges() {
		processing = true;
		try {
			let skinChanged = false;
			if (pendingSkin) {
				await uploadSkinFile(
					uuid,
					pendingSkin.filePath,
					pendingSkin.variant === "SLIM" ? "slim" : "classic",
				);
				skinChanged = true;
			} else if (skinModel !== originalVariant) {
				const activeSkin = profile?.skins.find(
					(s) => s.state === "ACTIVE",
				);
				if (activeSkin?.url) {
					await uploadSkinUrl(uuid, activeSkin.url, skinModel);
					skinChanged = true;
				}
			}

			if (selectedCapeId !== originalCapeId) {
				if (selectedCapeId === null) {
					await unequipCape(uuid);
				} else {
					await equipCape(uuid, selectedCapeId);
				}
			}

			const changedAnything =
				skinChanged || selectedCapeId !== originalCapeId;
			if (changedAnything) {
				showSuccess(
					t("userMenu.skinCape.changesSaved"),
					t("userMenu.skinCape.changesSavedDesc"),
				);
				bumpAvatarVersion(uuid);
			}

			discardChanges();
			await loadProfile(true);
		} catch (err) {
			if (isRateLimitError(err)) {
				showError(
					t("userMenu.skinCape.rateLimitTitle"),
					t("userMenu.skinCape.rateLimitMessage"),
				);
			} else {
				showError(t("errors.title"), String(err));
			}
		} finally {
			processing = false;
		}
	}

	const activeSkin = $derived(
		profile?.skins.find((s: MinecraftProfileSkin) => s.state === "ACTIVE"),
	);
	const activeCape = $derived(
		profile?.capes.find((c: MinecraftProfileCape) => c.state === "ACTIVE"),
	);

	const previewSkinUrl = $derived(
		pendingSkin?.previewUrl ?? activeSkin?.url ?? "",
	);
	const previewCapeUrl = $derived(
		profile?.capes.find((c) => c.id === selectedCapeId)?.url ?? null,
	);

	const hasPending = $derived(
		pendingSkin !== null ||
			selectedCapeId !== originalCapeId ||
			skinModel !== originalVariant,
	);

	$effect(() => {
		loadProfile();
	});

	onMount(() => {
		let mounted = true;
		let unlisten: (() => void) | null = null;

		async function setupDragDrop() {
			try {
				const { getCurrentWebview } =
					await import("@tauri-apps/api/webview");
				const webview = getCurrentWebview();
				unlisten = await webview.onDragDropEvent((event) => {
					if (!mounted) return;
					const payload = event.payload as {
						type: string;
						paths?: string[];
					};

					if (payload.type === "enter" || payload.type === "over") {
						const paths = payload.paths ?? [];
						draggingPng = paths.some((p) =>
							p.toLowerCase().endsWith(".png"),
						);
					} else if (payload.type === "leave") {
						draggingPng = false;
						dropTargetActive = false;
					} else if (payload.type === "drop") {
						const paths = payload.paths ?? [];
						if (dropTargetActive) {
							void handleDroppedSkinFile(paths);
						}
						draggingPng = false;
						dropTargetActive = false;
					}
				});
			} catch (e) {
				console.warn("Drag-drop not available:", e);
			}
		}

		setupDragDrop();

		return () => {
			mounted = false;
			unlisten?.();
		};
	});
</script>

<div class="skin-cape-manager">
	<div class="section-header">
		<h4 class="section-title">{t("userMenu.skinCape.title")}</h4>
		<button
			type="button"
			class="icon-btn"
			onclick={() => {
				discardChanges();
				void loadProfile(true);
			}}
			disabled={processing || loading}
			aria-label={t("userMenu.skinCape.loading")}
		>
			{#if loading}
				<span class="spinner"></span>
			{:else}
				<Icon name="ui:refresh" size={14} />
			{/if}
		</button>
	</div>

	{#if loading && !profile}
		<div class="loading-state">
			<span class="spinner"></span>
			<span>{t("userMenu.skinCape.loading")}</span>
		</div>
	{:else if profile}
		<div class="preview-card">
			<div class="model-selector">
				<button
					type="button"
					class="model-btn"
					class:active={skinModel === "classic"}
					onclick={() => handleModelChange("classic")}
					disabled={processing}
				>
					{t("userMenu.skinCape.classic")}
				</button>
				<button
					type="button"
					class="model-btn"
					class:active={skinModel === "slim"}
					onclick={() => handleModelChange("slim")}
					disabled={processing}
				>
					{t("userMenu.skinCape.slim")}
				</button>
			</div>

			<SkinPreview
				skinUrl={previewSkinUrl}
				capeUrl={previewCapeUrl}
				model={skinModel === "slim" ? "slim" : "default"}
				variant={pendingSkin?.variant ?? activeSkin?.variant}
				{draggingPng}
				{dropTargetActive}
				onDragEnter={(e) => {
					e.preventDefault();
					dropTargetActive = true;
				}}
				onDragLeave={(e) => {
					e.preventDefault();
					dropTargetActive = false;
				}}
				onDragOver={(e) => e.preventDefault()}
				onDrop={(e) => e.preventDefault()}
			/>

			{#if hasPending}
				<div
					class="save-actions-float"
					in:fly={{ y: 10, duration: 200 }}
				>
					<button
						type="button"
						class="icon-btn discard-icon"
						onclick={discardChanges}
						disabled={processing}
						aria-label={t("userMenu.skinCape.discardChanges")}
						title={t("userMenu.skinCape.discardChanges")}
					>
						<Icon name="ui:close" size={14} />
					</button>
					<button
						type="button"
						class="icon-btn save-icon"
						onclick={handleSaveChanges}
						disabled={processing}
						aria-label={t("userMenu.skinCape.saveChanges")}
						title={t("userMenu.skinCape.saveChanges")}
					>
						{#if processing}
							<span class="spinner"></span>
						{:else}
							<Icon name="ui:check" size={14} />
						{/if}
					</button>
				</div>
			{/if}
		</div>

		<CapeList
			capes={profile.capes}
			{selectedCapeId}
			activeCapeId={activeCape?.id ?? null}
			{processing}
			onSelect={handleSelectCape}
		/>

		<div class="upload-panel">
			<button
				type="button"
				class="upload-btn"
				onclick={() => handleFileUpload()}
				disabled={processing}
			>
				<Icon name="ui:upload" size={14} />
				<span>{t("userMenu.skinCape.uploadSkin")}</span>
			</button>
		</div>
	{/if}
</div>

<style>
	.skin-cape-manager {
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: 100%;
		min-height: 0;
		flex: 1;
	}

	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding-bottom: 8px;
		border-bottom: 1px solid var(--border);
	}

	.section-title {
		margin: 0;
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 0.6px;
	}

	.icon-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		border-radius: var(--border-radius-sm);
		width: 24px;
		height: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition:
			background 0.15s ease,
			color 0.15s ease;
	}

	.icon-btn:hover:not(:disabled) {
		background: var(--surface-selected);
		color: var(--text-primary);
	}

	.icon-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.loading-state {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 10px;
		color: var(--text-secondary);
		font-size: 0.8rem;
		padding: 24px;
	}

	.spinner {
		width: 14px;
		height: 14px;
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

	.preview-card {
		flex: 1;
		min-height: 0;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		overflow: hidden;
		position: relative;
		display: flex;
		flex-direction: column;
	}

	.preview-card :global(.preview-zone) {
		flex: 1;
		min-height: auto;
		border: none;
		border-radius: 0;
		height: 100%;
	}

	.model-selector {
		position: absolute;
		top: 12px;
		right: 12px;
		z-index: 5;
		display: inline-flex;
		gap: 4px;
		background: rgba(var(--bg-card-rgb, 0, 0, 0), 0.6);
		backdrop-filter: blur(4px);
		border-radius: var(--border-radius-sm);
		padding: 3px;
		border: 1px solid rgba(255, 255, 255, 0.08);
	}

	.model-btn {
		background: transparent;
		border: 1px solid transparent;
		color: var(--text-secondary);
		padding: 4px 10px;
		border-radius: var(--border-radius-sm);
		font-family: inherit;
		font-size: 0.65rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.4px;
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
		background: rgba(var(--accent-rgb), 0.15);
		border-color: var(--accent);
		color: var(--accent);
	}

	.model-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.upload-panel {
		display: flex;
		justify-content: center;
		padding: 4px 0;
	}

	.upload-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 6px 12px;
		border-radius: var(--border-radius-sm);
		font-family: inherit;
		font-size: 0.72rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.15s ease,
			border-color 0.15s ease,
			color 0.15s ease;
		background: var(--bg-card);
		color: var(--text-secondary);
		border: 1px solid var(--border);
	}

	.upload-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.upload-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.save-actions-float {
		position: absolute;
		bottom: 12px;
		right: 12px;
		z-index: 5;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.save-actions-float .icon-btn {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			opacity 0.15s ease;
	}

	.save-actions-float .icon-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.save-actions-float .icon-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.save-actions-float .save-icon {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--accent-text);
	}

	.save-actions-float .save-icon:hover:not(:disabled) {
		background: var(--accent-hover);
		border-color: var(--accent-hover);
	}

	.save-actions-float .spinner {
		width: 14px;
		height: 14px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@media (max-width: 520px) {
		.preview-card :global(.preview-zone) {
			min-height: 260px;
			height: 260px;
		}

		.model-selector {
			top: 8px;
			right: 8px;
		}

		.save-actions-float {
			bottom: 8px;
			right: 8px;
		}
	}
</style>
