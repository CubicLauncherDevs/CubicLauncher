<script lang="ts">
	import { onMount } from "svelte";
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
	import PendingChangesBar from "./PendingChangesBar.svelte";
	import SkinPreview from "./SkinPreview.svelte";
	import SkinUploadControls from "./SkinUploadControls.svelte";
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
	let skinModel = $state<"classic" | "slim">("classic");
	let skinUrlInput = $state("");
	let processing = $state(false);
	let showUrl = $state(false);
	let draggingPng = $state(false);
	let dropTargetActive = $state(false);

	interface PendingSkin {
		source: "file" | "url";
		filePath?: string;
		url?: string;
		variant: "CLASSIC" | "SLIM";
		previewUrl?: string;
	}

	type PendingCape = { type: "equip"; capeId: string } | { type: "unequip" };

	let pendingSkin = $state<PendingSkin | null>(null);
	let pendingCape = $state<PendingCape | null>(null);

	const hasPending = $derived(pendingSkin !== null || pendingCape !== null);

	async function loadProfile(silent = false) {
		if (!silent) loading = true;
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
		if (!silent) loading = false;
	}

	function discardChanges() {
		pendingSkin = null;
		pendingCape = null;
		skinUrlInput = "";
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
		let previewUrl: string | undefined;
		try {
			previewUrl = await getSkinPreviewData(selected);
		} catch (e) {
			showError(t("errors.title"), String(e));
			return;
		}
		pendingSkin = {
			source: "file",
			filePath: selected,
			variant: skinModel === "slim" ? "SLIM" : "CLASSIC",
			previewUrl,
		};
	}

	async function handleUrlUpload() {
		const url = skinUrlInput.trim();
		if (!url) return;
		pendingSkin = {
			source: "url",
			url,
			variant: skinModel === "slim" ? "SLIM" : "CLASSIC",
		};
		skinUrlInput = "";
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

	async function handleSaveChanges() {
		processing = true;
		try {
			if (pendingSkin) {
				if (pendingSkin.source === "file" && pendingSkin.filePath) {
					await uploadSkinFile(
						uuid,
						pendingSkin.filePath,
						pendingSkin.variant === "SLIM" ? "slim" : "classic",
					);
				} else if (pendingSkin.url) {
					await uploadSkinUrl(
						uuid,
						pendingSkin.url,
						pendingSkin.variant,
					);
				}
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
			await loadProfile(true);
			bumpAvatarVersion(uuid);
		} catch (e) {
			showError(t("errors.title"), String(e));
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
		pendingSkin?.previewUrl ?? pendingSkin?.url ?? activeSkin?.url ?? "",
	);
	const previewModel = $derived(
		pendingSkin?.variant === "SLIM" ? "slim" : viewerModel,
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
				<Icon src="/images/icons/ui/refresh.svg" size={16} />
			{/if}
		</button>
	</div>

	{#if hasPending}
		<PendingChangesBar
			{processing}
			onSave={handleSaveChanges}
			onDiscard={discardChanges}
		/>
	{/if}

	{#if loading && !profile}
		<div class="loading-state">
			<span class="spinner"></span>
			<span>{t("userMenu.skinCape.loading")}</span>
		</div>
	{:else if profile}
		<SkinPreview
			skinUrl={previewSkinUrl}
			capeUrl={previewCapeUrl}
			model={previewModel}
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

		<SkinUploadControls
			{skinModel}
			{showUrl}
			bind:skinUrlInput
			{processing}
			onModelChange={handleModelChange}
			onFileSelect={() => handleFileUpload()}
			onUrlToggle={() => (showUrl = !showUrl)}
			onUrlSubmit={handleUrlUpload}
		/>

		<CapeList
			capes={profile.capes}
			activeCapeId={previewActiveCapeId}
			showUnequipPending={pendingCape?.type === "unequip"}
			{processing}
			onEquip={handleEquipCape}
			onUnequip={handleUnequipCape}
		/>
	{/if}
</div>

<style>
	.skin-cape-manager {
		display: flex;
		flex-direction: column;
		gap: 14px;
		width: 100%;
	}

	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}

	.section-title {
		margin: 0;
		font-size: 0.85rem;
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
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.15s ease;
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
		font-size: 0.85rem;
		padding: 24px;
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
