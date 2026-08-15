<script lang="ts">
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

	async function handleFileUpload() {
		const selected = await openDialog({
			multiple: false,
			filters: [
				{
					name: "Minecraft Skin",
					extensions: ["png"],
				},
			],
		});
		if (!selected || Array.isArray(selected)) return;
		processing = true;
		try {
			await uploadSkinFile(uuid, selected, skinModel);
			showSuccess(
				t("userMenu.skinCape.skinUpdated"),
				t("userMenu.skinCape.skinUpdatedDesc"),
			);
			await loadProfile(true);
		} catch (e) {
			showError(t("errors.title"), String(e));
		} finally {
			processing = false;
		}
	}

	async function handleUrlUpload() {
		const url = skinUrlInput.trim();
		if (!url) return;
		processing = true;
		try {
			await uploadSkinUrl(
				uuid,
				url,
				skinModel === "slim" ? "SLIM" : "CLASSIC",
			);
			showSuccess(
				t("userMenu.skinCape.skinUpdated"),
				t("userMenu.skinCape.skinUpdatedDesc"),
			);
			skinUrlInput = "";
			await loadProfile(true);
		} catch (e) {
			showError(t("errors.title"), String(e));
		} finally {
			processing = false;
		}
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

	$effect(() => {
		loadProfile();
	});
</script>

<div class="skin-cape-manager">
	<div class="section-header">
		<h4 class="section-title">{t("userMenu.skinCape.title")}</h4>
		{#if loading}
			<span class="spinner"></span>
		{:else}
			<button
				type="button"
				class="refresh-btn"
				onclick={() => loadProfile(true)}
				disabled={processing}
			>
				↻
			</button>
		{/if}
	</div>

	{#if loading && !profile}
		<div class="loading-state">
			<span class="spinner"></span>
			<span>{t("userMenu.skinCape.loading")}</span>
		</div>
	{:else if profile}
		<div class="preview-zone">
			<div class="preview-card">
				<div class="preview-meta">
					<span class="preview-label">
						{t("userMenu.skinCape.currentSkin")}
					</span>
					<span class="preview-variant">
						{activeSkin?.variant ?? "CLASSIC"}
					</span>
				</div>
			</div>

			{#if activeSkin}
				<div class="skin-3d-preview">
					<Skin3dViewer
						skinUrl={activeSkin.url}
						capeUrl={activeCape?.url}
						model={viewerModel}
					/>
				</div>
			{/if}
		</div>

		<div class="skin-controls">
			<div class="model-toggle">
				<button
					type="button"
					class="model-btn"
					class:active={skinModel === "classic"}
					onclick={() => (skinModel = "classic")}
				>
					{t("userMenu.skinCape.classic")}
				</button>
				<button
					type="button"
					class="model-btn"
					class:active={skinModel === "slim"}
					onclick={() => (skinModel = "slim")}
				>
					{t("userMenu.skinCape.slim")}
				</button>
			</div>

			<button
				type="button"
				class="btn-primary"
				onclick={handleFileUpload}
				disabled={processing}
			>
				{t("userMenu.skinCape.uploadSkin")}
			</button>

			<div class="url-row">
				<input
					type="text"
					bind:value={skinUrlInput}
					placeholder={t("userMenu.skinCape.skinUrlPlaceholder")}
					class="url-input"
					onkeydown={(e) => e.key === "Enter" && handleUrlUpload()}
				/>
				<button
					type="button"
					class="btn-secondary"
					onclick={handleUrlUpload}
					disabled={processing || !skinUrlInput.trim()}
				>
					{t("userMenu.skinCape.useUrl")}
				</button>
			</div>
		</div>

		<div class="capes-section">
			<h5 class="subsection-title">{t("userMenu.skinCape.capes")}</h5>
			{#if !hasCapes}
				<p class="empty-text">{t("userMenu.skinCape.noCapes")}</p>
			{:else}
				{#if activeCape}
					<div class="active-cape-card">
						<div class="active-cape-preview">
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
						</div>
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
							class="btn-secondary"
							onclick={handleUnequipCape}
							disabled={processing}
						>
							{t("userMenu.skinCape.unequip")}
						</button>
					</div>
				{/if}

				<div class="cape-list">
					{#each profile.capes.filter((c: MinecraftProfileCape) => c.state !== "ACTIVE") as cape (cape.id)}
						<div class="cape-item">
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
									{cape.alias || t("userMenu.skinCape.cape")}
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
		</div>
	{/if}
</div>

<style>
	.skin-cape-manager {
		display: flex;
		flex-direction: column;
		gap: 18px;
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

	.refresh-btn {
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
		font-size: 0.9rem;
		transition: all 0.15s ease;
	}

	.refresh-btn:hover:not(:disabled) {
		background: var(--surface-selected);
		color: var(--text-primary);
	}

	.refresh-btn:disabled {
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

	.preview-zone {
		display: flex;
		gap: 16px;
		align-items: flex-start;
		flex-wrap: wrap;
	}

	.preview-card {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		padding: 16px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
		min-width: 160px;
	}

	.preview-meta {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
	}

	.preview-label {
		font-size: 0.72rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.4px;
	}

	.preview-variant {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--accent);
	}

	.skin-3d-preview {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		padding: 12px;
		display: flex;
		align-items: center;
		justify-content: center;
		min-width: 180px;
		min-height: 260px;
		flex: 1;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
	}

	.skin-controls {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.model-toggle {
		display: flex;
		gap: 0;
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		border: 1px solid var(--border);
		width: fit-content;
	}

	.model-btn {
		background: var(--bg-input);
		border: none;
		color: var(--text-secondary);
		padding: 8px 16px;
		font-family: inherit;
		font-size: 0.78rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s ease;
		min-width: 80px;
	}

	.model-btn:first-child {
		border-right: 1px solid var(--border);
	}

	.model-btn.active {
		background: var(--accent);
		color: var(--accent-text);
	}

	.model-btn:hover:not(.active) {
		background: var(--surface-selected);
		color: var(--text-primary);
	}

	.url-row {
		display: flex;
		gap: 8px;
		align-items: center;
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

	.btn-primary,
	.btn-secondary {
		font-family: inherit;
		font-size: 0.78rem;
		font-weight: 600;
		padding: 8px 16px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition: all 0.15s ease;
		border: 1px solid transparent;
		white-space: nowrap;
	}

	.btn-primary {
		background: var(--accent);
		color: var(--accent-text);
		width: fit-content;
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
	}

	.empty-text {
		margin: 0;
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.active-cape-card {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		padding: 14px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		box-shadow:
			var(--shadow-sm),
			inset 0 1px 0 var(--surface-selected);
	}

	.active-cape-preview {
		width: 100%;
		max-width: 320px;
		aspect-ratio: 2 / 1;
		background: var(--bg-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.active-cape-img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		image-rendering: pixelated;
	}

	.active-cape-img-fallback {
		background: var(--cubic-logo) center/25% no-repeat;
	}

	.active-cape-meta {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
	}

	.active-cape-name {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-primary);
		text-align: center;
		word-break: break-word;
	}

	.active-cape-status {
		font-size: 0.62rem;
		font-weight: 700;
		color: var(--accent);
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	.cape-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.cape-item {
		display: flex;
		align-items: center;
		gap: 12px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 10px 12px;
		transition: background 0.15s ease;
	}

	.cape-item:hover {
		background: var(--surface-selected);
	}

	.cape-thumb {
		width: 80px;
		height: 40px;
		object-fit: contain;
		image-rendering: pixelated;
		border-radius: 4px;
		background: var(--bg-input);
		border: 1px solid var(--border);
		flex-shrink: 0;
		transition: transform 0.15s ease;
	}

	.cape-item:hover .cape-thumb {
		transform: scale(1.05);
	}

	.cape-thumb-fallback {
		background: var(--cubic-logo) center/35% no-repeat;
	}

	.cape-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.cape-name {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.cape-action {
		flex-shrink: 0;
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
