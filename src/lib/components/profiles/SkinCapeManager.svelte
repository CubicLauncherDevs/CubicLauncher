<script lang="ts">
	import { onMount } from "svelte";
	import { t } from "$lib/i18n";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";
	import {
		getMinecraftProfile,
		uploadSkinFile,
		uploadSkinUrl,
		equipCape,
		unequipCape,
	} from "$lib/api/cubicApi";
	import { showError, showSuccess } from "$lib/state/state.svelte";
	import { bumpAvatarVersion } from "$lib/state/avatarCache.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import Skin3dViewer from "./Skin3dViewer.svelte";
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

	async function applySkinUpload(upload: () => Promise<void>) {
		processing = true;
		try {
			await upload();
			showSuccess(
				t("userMenu.skinCape.skinUpdated"),
				t("userMenu.skinCape.skinUpdatedDesc"),
			);
			await loadProfile(true);
			bumpAvatarVersion(uuid);
		} catch (e) {
			showError(t("errors.title"), String(e));
		} finally {
			processing = false;
		}
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
		await applySkinUpload(() => uploadSkinFile(uuid, selected, skinModel));
	}

	async function handleUrlUpload() {
		const url = skinUrlInput.trim();
		if (!url) return;
		await applySkinUpload(() =>
			uploadSkinUrl(uuid, url, skinModel === "slim" ? "SLIM" : "CLASSIC"),
		);
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

	async function handleEquipCape(capeId: string) {
		processing = true;
		try {
			await equipCape(uuid, capeId);
			showSuccess(
				t("userMenu.skinCape.capeUpdated"),
				t("userMenu.skinCape.capeUpdatedDesc"),
			);
			await loadProfile(true);
		} catch (e) {
			showError(t("errors.title"), String(e));
		} finally {
			processing = false;
		}
	}

	async function handleUnequipCape() {
		processing = true;
		try {
			await unequipCape(uuid);
			showSuccess(
				t("userMenu.skinCape.capeUpdated"),
				t("userMenu.skinCape.capeUpdatedDesc"),
			);
			await loadProfile(true);
		} catch (e) {
			showError(t("errors.title"), String(e));
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
	const viewerModel = $derived(
		activeSkin?.variant === "SLIM" ? "slim" : "default",
	);
	const hasCapes = $derived((profile?.capes.length ?? 0) > 0);
	const inactiveCapes = $derived(
		profile?.capes.filter(
			(c: MinecraftProfileCape) => c.state !== "ACTIVE",
		) ?? [],
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
			onclick={() => loadProfile(true)}
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

	{#if loading && !profile}
		<div class="loading-state">
			<span class="spinner"></span>
			<span>{t("userMenu.skinCape.loading")}</span>
		</div>
	{:else if profile}
		<div
			class="preview-zone"
			class:drop-ready={draggingPng && dropTargetActive}
			role="button"
			tabindex="0"
			ondragenter={(e) => {
				e.preventDefault();
				dropTargetActive = true;
			}}
			ondragleave={(e) => {
				e.preventDefault();
				dropTargetActive = false;
			}}
			ondragover={(e) => e.preventDefault()}
		>
			{#if activeSkin}
				<span class="model-badge">
					{activeSkin.variant}
				</span>

				<Skin3dViewer
					skinUrl={activeSkin.url}
					capeUrl={activeCape?.url}
					model={viewerModel}
				/>

				{#if draggingPng && dropTargetActive}
					<div class="drop-overlay">
						<Icon src="/images/icons/ui/upload.svg" size={32} />
						<span>{t("userMenu.skinCape.dropSkinHere")}</span>
					</div>
				{/if}
			{:else}
				<div class="empty-preview">
					<span>No hay skin activa</span>
				</div>
			{/if}
		</div>

		<div class="model-selector">
			<button
				type="button"
				class="model-btn"
				class:active={skinModel === "classic"}
				onclick={() => (skinModel = "classic")}
				disabled={processing}
			>
				<span class="model-label">{t("userMenu.skinCape.classic")}</span
				>
				{#if skinModel === "classic"}
					<Icon src="/images/icons/ui/check.svg" size={14} />
				{/if}
			</button>
			<button
				type="button"
				class="model-btn"
				class:active={skinModel === "slim"}
				onclick={() => (skinModel = "slim")}
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
				onclick={() => handleFileUpload()}
				disabled={processing}
			>
				<Icon src="/images/icons/ui/upload.svg" size={16} />
				<span>{t("userMenu.skinCape.uploadSkin")}</span>
			</button>
			<button
				type="button"
				class="btn-secondary url-toggle"
				onclick={() => (showUrl = !showUrl)}
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
					onkeydown={(e) => e.key === "Enter" && handleUrlUpload()}
					disabled={processing}
				/>
				<button
					type="button"
					class="btn-primary"
					onclick={handleUrlUpload}
					disabled={processing || !skinUrlInput.trim()}
				>
					{t("userMenu.skinCape.useUrl")}
				</button>
			</div>
		{/if}

		<div class="capes-section">
			<h5 class="subsection-title">
				{t("userMenu.skinCape.capes")}
				{#if hasCapes}
					<span class="cape-count">{profile?.capes.length}</span>
				{/if}
			</h5>

			{#if !hasCapes}
				<p class="empty-text">{t("userMenu.skinCape.noCapes")}</p>
			{:else}
				{#if activeCape}
					<div class="active-cape-card">
						{#if activeCape.url}
							<img
								src={activeCape.url}
								alt={activeCape.alias}
								class="active-cape-img"
							/>
						{:else}
							<div
								class="active-cape-img active-cape-img-fallback"
							></div>
						{/if}
						<div class="active-cape-meta">
							<span class="active-cape-name">
								{activeCape.alias ||
									t("userMenu.skinCape.cape")}
							</span>
							<span class="active-cape-status">
								{t("userMenu.skinCape.active")}
							</span>
						</div>
						<button
							type="button"
							class="btn-secondary unequip-btn"
							onclick={handleUnequipCape}
							disabled={processing}
						>
							{t("userMenu.skinCape.unequip")}
						</button>
					</div>
				{/if}

				{#if inactiveCapes.length > 0}
					<div class="cape-grid">
						{#each inactiveCapes as cape (cape.id)}
							<div class="cape-card">
								{#if cape.url}
									<img
										src={cape.url}
										alt={cape.alias}
										class="cape-thumb"
									/>
								{:else}
									<div
										class="cape-thumb cape-thumb-fallback"
									></div>
								{/if}
								<div class="cape-info">
									<span class="cape-name">
										{cape.alias ||
											t("userMenu.skinCape.cape")}
									</span>
								</div>
								<button
									type="button"
									class="btn-primary cape-action"
									onclick={() => handleEquipCape(cape.id)}
									disabled={processing}
								>
									{t("userMenu.skinCape.equip")}
								</button>
							</div>
						{/each}
					</div>
				{/if}
			{/if}
		</div>
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

	.preview-zone {
		position: relative;
		background: var(--bg-card);
		border: 2px solid var(--border);
		border-radius: var(--border-radius);
		overflow: hidden;
		min-height: 280px;
		height: 320px;
		transition:
			border-color 0.15s ease,
			background 0.15s ease;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
		outline: none;
	}

	.preview-zone:focus-visible {
		border-color: var(--accent);
	}

	.preview-zone.drop-ready {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.04);
	}

	.model-badge {
		position: absolute;
		top: 10px;
		right: 10px;
		z-index: 3;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 3px 8px;
		font-size: 0.62rem;
		font-weight: 700;
		color: var(--accent);
		text-transform: uppercase;
		letter-spacing: 0.4px;
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
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 10px 16px;
		font-size: 0.8rem;
	}

	.url-toggle {
		padding: 10px 14px;
		font-size: 0.78rem;
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

	.capes-section {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding-top: 8px;
		border-top: 1px solid var(--border);
	}

	.subsection-title {
		margin: 0;
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.cape-count {
		background: var(--surface-selected);
		color: var(--text-secondary);
		padding: 1px 6px;
		border-radius: 999px;
		font-size: 0.65rem;
	}

	.empty-text {
		margin: 0;
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.active-cape-card {
		display: flex;
		align-items: center;
		gap: 12px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-left: 3px solid var(--accent);
		border-radius: var(--border-radius);
		padding: 12px;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
	}

	.active-cape-img {
		width: 70px;
		height: 35px;
		object-fit: contain;
		image-rendering: pixelated;
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		border: 1px solid var(--border);
		flex-shrink: 0;
	}

	.active-cape-img-fallback {
		background: var(--cubic-logo) center/30% no-repeat;
	}

	.active-cape-meta {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.active-cape-name {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.active-cape-status {
		font-size: 0.62rem;
		font-weight: 700;
		color: var(--accent);
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	.unequip-btn {
		flex-shrink: 0;
		font-size: 0.75rem;
		padding: 6px 12px;
	}

	.cape-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 10px;
	}

	.cape-card {
		display: flex;
		flex-direction: column;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		overflow: hidden;
		transition: background 0.15s ease;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
	}

	.cape-card:hover {
		background: var(--surface-selected);
	}

	.cape-thumb {
		width: 100%;
		height: 70px;
		object-fit: contain;
		image-rendering: pixelated;
		background: var(--bg-input);
		border-bottom: 1px solid var(--border);
		padding: 6px;
	}

	.cape-thumb-fallback {
		background: var(--cubic-logo) center/30% no-repeat;
	}

	.cape-info {
		padding: 10px;
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.cape-name {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.cape-action {
		margin: 0 10px 10px;
		font-size: 0.75rem;
		padding: 6px 10px;
	}

	.btn-primary,
	.btn-secondary {
		font-family: inherit;
		font-weight: 600;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition: all 0.15s ease;
		border: 1px solid transparent;
		white-space: nowrap;
	}

	.btn-primary {
		background: var(--accent);
		color: var(--accent-text);
	}

	.btn-primary:hover:not(:disabled) {
		opacity: 0.85;
	}

	.btn-secondary {
		background: transparent;
		border-color: var(--border);
		color: var(--text-secondary);
	}

	.btn-secondary:hover:not(:disabled) {
		background: var(--surface-selected);
		color: var(--text-primary);
	}

	.btn-primary:disabled,
	.btn-secondary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	@media (max-width: 520px) {
		.cape-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
