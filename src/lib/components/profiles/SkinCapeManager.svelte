<script lang="ts">
	import { onMount } from "svelte";
	import { t } from "$lib/i18n";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";
	import {
		getMinecraftProfile,
		getSkinPreviewData,
		uploadSkinFile,
		equipCape,
		unequipCape,
	} from "$lib/api/cubicApi";
	import { showError, showSuccess } from "$lib/state/state.svelte";
	import { bumpAvatarVersion } from "$lib/state/avatarCache.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import SkinPreview from "./SkinPreview.svelte";
	import SkinUploadControls from "./SkinUploadControls.svelte";
	import CapeList from "./CapeList.svelte";
	import SkinCloset from "./SkinCloset.svelte";
	import type {
		MinecraftProfileCape,
		MinecraftProfileResponse,
		MinecraftProfileSkin,
		SkinClosetEntry,
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

	interface PendingSkin {
		filePath: string;
		variant: "CLASSIC" | "SLIM";
		previewUrl: string;
	}

	type PendingCape = { type: "equip"; capeId: string } | { type: "unequip" };

	let pendingSkin = $state<PendingSkin | null>(null);
	let pendingCape = $state<PendingCape | null>(null);
	let closetRevision = $state(0);
	let equippedClosetSkin = $state<SkinClosetEntry | null>(null);

	const hasPending = $derived(pendingSkin !== null || pendingCape !== null);

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
				} else {
					skinModel = "classic";
				}
			}
		} finally {
			fetchingProfile = false;
			if (!silent) loading = false;
		}
	}

	function discardChanges() {
		pendingSkin = null;
		pendingCape = null;
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
		equippedClosetSkin = null;
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

	function handleEquipCape(capeId: string) {
		pendingCape = { type: "equip", capeId };
	}

	function handleUnequipCape() {
		pendingCape = { type: "unequip" };
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
			if (pendingSkin) {
				await uploadSkinFile(
					uuid,
					pendingSkin.filePath,
					pendingSkin.variant === "SLIM" ? "slim" : "classic",
				);
			}
			if (pendingCape) {
				if (pendingCape.type === "equip") {
					await equipCape(uuid, pendingCape.capeId);
				} else {
					await unequipCape(uuid);
				}
			}
			showSuccess(
				t("userMenu.skinCape.changesSaved"),
				t("userMenu.skinCape.changesSavedDesc"),
			);
			discardChanges();
			equippedClosetSkin = null;
			await loadProfile(true);
			closetRevision += 1;
			bumpAvatarVersion(uuid);
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

	function handleModelChange(model: "classic" | "slim") {
		skinModel = model;
		if (pendingSkin)
			pendingSkin.variant = model === "slim" ? "SLIM" : "CLASSIC";
	}

	const activeSkin = $derived(
		profile?.skins.find((s: MinecraftProfileSkin) => s.state === "ACTIVE"),
	);
	const activeCape = $derived(
		profile?.capes.find((c: MinecraftProfileCape) => c.state === "ACTIVE"),
	);
	const viewerModel = $derived(
		activeSkin?.variant === "SLIM" ? "slim" : "default",
	);

	const previewSkinUrl = $derived(
		pendingSkin?.previewUrl ??
			equippedClosetSkin?.url ??
			activeSkin?.url ??
			"",
	);
	const previewModel = $derived(
		pendingSkin?.variant === "SLIM"
			? "slim"
			: equippedClosetSkin?.variant.toUpperCase() === "SLIM"
				? "slim"
				: viewerModel,
	);
	const previewCapeUrl = $derived.by(() => {
		const pc = pendingCape;
		if (pc?.type === "equip") {
			return profile?.capes.find((c) => c.id === pc.capeId)?.url ?? null;
		}
		if (pc?.type === "unequip") return null;
		return activeCape?.url ?? null;
	});
	const previewActiveCapeId = $derived.by(() => {
		const pc = pendingCape;
		if (pc?.type === "equip") return pc.capeId;
		if (pc?.type === "unequip") return null;
		return activeCape?.id ?? null;
	});

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
		<div class="content-grid">
			<div class="left-col">
				<div class="viewer-card">
					<div class="preview-panel">
						<SkinPreview
							skinUrl={previewSkinUrl}
							capeUrl={previewCapeUrl}
							model={previewModel}
							variant={pendingSkin?.variant ??
								activeSkin?.variant}
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
					</div>

					<div class="controls-panel">
						<SkinUploadControls
							{skinModel}
							{processing}
							onModelChange={handleModelChange}
							onFileSelect={() => handleFileUpload()}
						/>
					</div>
				</div>
			</div>

			<div class="right-col">
				<div class="capes-panel">
					<CapeList
						capes={profile.capes}
						activeCapeId={previewActiveCapeId}
						showUnequipPending={pendingCape?.type === "unequip"}
						{processing}
						onEquip={handleEquipCape}
						onUnequip={handleUnequipCape}
					/>
				</div>
			</div>
		</div>

		<SkinCloset
			{uuid}
			activeSkinId={equippedClosetSkin?.id ?? activeSkin?.id ?? null}
			processing={processing || loading}
			refreshTrigger={closetRevision}
			onEquipped={(entry) => {
				equippedClosetSkin = entry;
				bumpAvatarVersion(uuid);
			}}
		/>
	{/if}
</div>

{#if hasPending}
	<div class="save-bar">
		<span class="pending-info">
			{t("userMenu.skinCape.pendingChanges")}
		</span>
		<div class="save-actions">
			<button
				type="button"
				class="btn-secondary discard-btn"
				onclick={discardChanges}
				disabled={processing}
			>
				{t("userMenu.skinCape.discardChanges")}
			</button>
			<button
				type="button"
				class="btn-primary save-btn"
				onclick={handleSaveChanges}
				disabled={processing}
			>
				{#if processing}
					<span class="spinner"></span>
				{/if}
				{t("userMenu.skinCape.saveChanges")}
			</button>
		</div>
	</div>
{/if}

<style>
	.skin-cape-manager {
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: 100%;
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

	.content-grid {
		display: grid;
		grid-template-columns: 1fr 320px;
		gap: 14px;
		align-items: start;
	}

	.left-col,
	.right-col {
		display: flex;
		flex-direction: column;
		gap: 12px;
		min-width: 0;
	}

	.right-col {
		position: sticky;
		top: 0;
		max-height: 100%;
	}

	.viewer-card {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.preview-panel :global(.preview-zone) {
		border: none;
		border-radius: 0;
		box-shadow: inset 0 -1px 0 0 var(--border);
	}

	.preview-panel :global(.preview-zone.drop-ready) {
		box-shadow:
			inset 0 -1px 0 0 var(--border),
			inset 0 0 0 2px var(--accent);
	}

	.controls-panel {
		padding: 12px;
		border-top: 1px solid var(--border);
	}

	.capes-panel {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		overflow: hidden;
		overflow-y: auto;
	}

	@media (max-width: 720px) {
		.content-grid {
			grid-template-columns: 1fr;
		}

		.right-col {
			position: static;
			max-height: none;
		}
	}

	.save-bar {
		position: sticky;
		bottom: 12px;
		width: fit-content;
		min-width: 360px;
		margin: 16px auto 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 12px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: 999px;
		padding: 6px 8px 6px 14px;
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
	}

	.pending-info {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		white-space: nowrap;
	}

	.save-actions {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.save-btn,
	.discard-btn {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 0.7rem;
		padding: 4px 10px;
		border-radius: var(--border-radius-sm);
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			opacity 0.15s ease;
	}

	.save-btn {
		background: var(--accent);
		color: var(--accent-text);
		border: 1px solid var(--accent);
	}

	.save-btn:hover:not(:disabled) {
		background: var(--accent-hover);
	}

	.discard-btn {
		background: transparent;
		color: var(--text-secondary);
		border: 1px solid var(--border);
	}

	.discard-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.save-btn:disabled,
	.discard-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.save-actions .spinner {
		width: 12px;
		height: 12px;
		margin: 0;
	}
</style>
