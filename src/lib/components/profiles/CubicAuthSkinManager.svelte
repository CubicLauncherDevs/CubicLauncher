<script lang="ts">
	import { onMount } from "svelte";
	import { fly } from "svelte/transition";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";
	import {
		getSkinPreviewData,
		getYggdrasilSkinProfile,
		openUrl,
		uploadYggdrasilSkin,
		uploadYggdrasilSkinUrl,
	} from "$lib/api/cubicApi";
	import { catalogViewUrl, type CatalogSkin } from "$lib/api/mosaicarySkins";
	import {
		launcherStore,
		showError,
		showSuccess,
	} from "$lib/state/state.svelte";
	import { bumpAvatarVersion } from "$lib/state/avatarCache.svelte";
	import type { YggdrasilSkinProfile } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import {
		canUseSkinModel,
		detectSkinModelFromUrl,
		type SkinModel,
	} from "$lib/utils/skinModel";
	import Icon from "$lib/icons/Icon.svelte";
	import Skin3dViewer from "./Skin3dViewer.svelte";
	import SkinCatalogModal from "./SkinCatalogModal.svelte";

	let { uuid }: { uuid: string } = $props();

	let profile = $state<YggdrasilSkinProfile | null>(null);
	let loading = $state(false);
	let error = $state(false);
	let processing = $state(false);
	let refreshOnReturn = false;
	let disposed = false;

	let skinModel = $state<SkinModel>("classic");
	let originalVariant = $state<SkinModel>("classic");
	// Modelo leído de los píxeles de la skin activa: una skin delgada no se puede
	// guardar como clásica sin que aparezcan franjas negras en los brazos.
	let detectedModel = $state<SkinModel | null>(null);
	let pendingSkin = $state<{
		/** Archivo elegido en el disco, o `null` si la skin viene del catálogo. */
		filePath: string | null;
		/** PNG del catálogo, que el backend descarga al guardar. */
		sourceUrl: string | null;
		previewUrl: string;
	} | null>(null);
	let catalogOpen = $state(false);

	const previewSkinUrl = $derived(
		pendingSkin?.previewUrl ?? profile?.skin_url ?? "",
	);
	const classicLocked = $derived(detectedModel === "slim");
	const hasPending = $derived(pendingSkin !== null);

	async function refresh(updateAvatar = true) {
		if (loading || disposed) return;
		loading = true;
		error = false;
		try {
			const next = await getYggdrasilSkinProfile(uuid);
			if (disposed) return;
			profile = next;
			originalVariant = next.model;
			if (!pendingSkin) {
				skinModel = next.model;
				detectedModel = next.skin_url
					? await detectSkinModelFromUrl(next.skin_url)
					: null;
			}
			if (updateAvatar) bumpAvatarVersion(uuid);
		} catch {
			if (!disposed) error = true;
		} finally {
			if (!disposed) loading = false;
		}
	}

	async function pickSkin() {
		const selected = await openDialog({
			multiple: false,
			filters: [{ name: "Minecraft Skin", extensions: ["png"] }],
		});
		if (!selected || Array.isArray(selected)) return;
		let previewUrl: string;
		try {
			previewUrl = await getSkinPreviewData(selected);
		} catch (e) {
			showError(t("errors.title"), String(e));
			return;
		}
		// El modelo sale de la skin elegida, no de la que estaba puesta.
		const detected = await detectSkinModelFromUrl(previewUrl);
		detectedModel = detected;
		if (detected) skinModel = detected;
		pendingSkin = { filePath: selected, sourceUrl: null, previewUrl };
	}

	async function pickFromCatalog(skin: CatalogSkin) {
		// Mirarla no cuenta como descarga: eso pasa al guardar, con `skin.png`.
		const previewUrl = catalogViewUrl(skin.png);
		const detected = await detectSkinModelFromUrl(previewUrl);
		detectedModel = detected;
		skinModel = detected === "slim" ? "slim" : skin.model;
		pendingSkin = { filePath: null, sourceUrl: skin.png, previewUrl };
	}

	function handleModelChange(model: SkinModel) {
		if (!canUseSkinModel(detectedModel, model)) return;
		skinModel = model;
	}

	function discardChanges() {
		pendingSkin = null;
		skinModel = originalVariant;
		detectedModel = null;
		void refresh(false);
	}

	async function saveSkin() {
		if (!pendingSkin || processing) return;
		processing = true;
		try {
			if (pendingSkin.sourceUrl) {
				await uploadYggdrasilSkinUrl(
					uuid,
					pendingSkin.sourceUrl,
					skinModel,
				);
			} else if (pendingSkin.filePath) {
				await uploadYggdrasilSkin(
					uuid,
					pendingSkin.filePath,
					skinModel,
				);
			}
			pendingSkin = null;
			showSuccess(
				t("userMenu.skinCape.skinUpdated"),
				t("userMenu.skinCape.skinUpdatedDesc"),
			);
			await refresh();
		} catch (e) {
			showError(t("errors.title"), String(e));
		} finally {
			processing = false;
		}
	}

	async function changeSkinOnWeb() {
		refreshOnReturn = true;
		await openUrl("https://accounts.cubiclauncher.org/");
	}

	function handleFocus() {
		if (!refreshOnReturn || loading) return;
		refreshOnReturn = false;
		void refresh();
	}

	onMount(() => {
		void refresh(false);
		return () => {
			disposed = true;
		};
	});
</script>

<svelte:window onfocus={handleFocus} />

<section class="cubic-auth-skin" aria-label={t("userMenu.cubicAuthSkin.title")}>
	<div class="section-header">
		<h4>{t("userMenu.cubicAuthSkin.title")}</h4>
		<button
			type="button"
			class="btn-secondary refresh-btn"
			onclick={() => refresh()}
			disabled={loading || processing}
			title={t("userMenu.cubicAuthSkin.refresh")}
			aria-label={t("userMenu.cubicAuthSkin.refresh")}
		>
			<Icon name="ui:refresh" size={16} />
		</button>
	</div>
	{#if error}
		<p class="load-error" role="alert">
			{t("userMenu.cubicAuthSkin.loadError")}
		</p>
	{/if}
	<div class="preview" aria-busy={loading}>
		{#if previewSkinUrl}
			<Skin3dViewer
				skinUrl={previewSkinUrl}
				capeUrl={null}
				model={skinModel === "slim" ? "slim" : "default"}
				quality={launcherStore.settings.reduce_skin3d_quality
					? "low"
					: "high"}
			/>
			<div class="model-controls">
				<div class="model-selector">
					<button
						type="button"
						class="model-btn"
						class:active={skinModel === "classic"}
						onclick={() => handleModelChange("classic")}
						disabled={processing || classicLocked}
						title={classicLocked
							? t("userMenu.skinCape.classicLocked")
							: undefined}
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
			</div>
			{#if hasPending}
				<div class="save-actions" in:fly={{ y: 10, duration: 200 }}>
					<button
						type="button"
						class="icon-btn"
						onclick={discardChanges}
						disabled={processing}
						aria-label={t("userMenu.skinCape.discardChanges")}
						title={t("userMenu.skinCape.discardChanges")}
					>
						<Icon name="ui:close" size={14} />
					</button>
					<button
						type="button"
						class="icon-btn save"
						onclick={saveSkin}
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
		{:else}
			<p class="preview-status" role="status">
				{loading
					? t("userMenu.skinCape.loading")
					: error
						? t("userMenu.cubicAuthSkin.unavailable")
						: t("userMenu.cubicAuthSkin.noSkin")}
			</p>
		{/if}
		<div class="preview-ctas">
			<button
				type="button"
				class="upload-cta"
				onclick={pickSkin}
				disabled={processing}
				title={t("userMenu.skinCape.uploadSkin")}
			>
				<Icon name="ui:upload" size={15} />
				<span>{t("userMenu.skinCape.uploadHint")}</span>
			</button>
			<button
				type="button"
				class="upload-cta catalog-cta"
				onclick={() => (catalogOpen = true)}
				disabled={processing}
				title={t("userMenu.skinCape.catalog.title")}
			>
				<Icon name="ui:search" size={15} />
				<span>{t("userMenu.skinCape.catalog.open")}</span>
			</button>
		</div>
	</div>
	{#if classicLocked}
		<div class="model-notice" role="note">
			<Icon name="ui:lock" size={14} color="var(--accent)" />
			<span>{t("userMenu.skinCape.modelLockedSlim")}</span>
		</div>
	{/if}
	<p class="help-text">{t("userMenu.cubicAuthSkin.description")}</p>
	<button
		type="button"
		class="btn-secondary change-btn"
		onclick={changeSkinOnWeb}
	>
		<Icon name="instance:external-link" size={16} />
		{t("userMenu.cubicAuthSkin.change")}
	</button>
	<p class="help-text">{t("userMenu.cubicAuthSkin.refreshHint")}</p>

	<SkinCatalogModal bind:open={catalogOpen} onPick={pickFromCatalog} />
</section>

<style>
	.cubic-auth-skin {
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

	h4 {
		margin: 0;
		font-size: 0.85rem;
		color: var(--text-primary);
	}

	.refresh-btn {
		padding: 6px;
	}

	.preview {
		position: relative;
		height: 300px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		overflow: hidden;
	}

	.preview-status {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		margin: 0;
		padding: 16px;
		text-align: center;
		color: var(--text-secondary);
	}

	.model-controls {
		position: absolute;
		top: 12px;
		right: 12px;
		z-index: 5;
	}

	.model-selector {
		display: inline-flex;
		gap: 4px;
		background: rgba(var(--bg-card-rgb, 0, 0, 0), 0.6);
		backdrop-filter: blur(4px);
		border-radius: var(--border-radius-sm);
		padding: 3px;
		border: var(--border-width) solid var(--border);
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

	.preview-ctas {
		position: absolute;
		left: 12px;
		bottom: 12px;
		z-index: 5;
		display: flex;
		gap: 6px;
		max-width: calc(100% - 108px);
	}

	.catalog-cta {
		flex-shrink: 0;
	}

	.upload-cta {
		min-width: 0;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 7px 12px;
		border-radius: var(--border-radius-sm);
		font-family: inherit;
		font-size: 0.7rem;
		font-weight: 600;
		cursor: pointer;
		background: rgba(var(--bg-card-rgb, 0, 0, 0), 0.72);
		backdrop-filter: blur(4px);
		border: 1px solid var(--border);
		color: var(--text-primary);
		transition:
			background 0.15s ease,
			border-color 0.15s ease,
			color 0.15s ease;
	}

	.upload-cta span {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.upload-cta:hover:not(:disabled) {
		border-color: var(--accent);
		color: var(--accent);
	}

	.upload-cta:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.save-actions {
		position: absolute;
		right: 12px;
		bottom: 12px;
		z-index: 5;
		display: flex;
		gap: 6px;
	}

	.icon-btn {
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
			color 0.15s ease;
	}

	.icon-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.icon-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.icon-btn.save {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--accent-text);
	}

	.icon-btn.save:hover:not(:disabled) {
		background: var(--accent-hover);
		border-color: var(--accent-hover);
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

	.model-notice {
		display: flex;
		align-items: flex-start;
		gap: 8px;
		padding: 8px 10px;
		border: 1px solid rgba(var(--accent-rgb), 0.35);
		border-radius: var(--border-radius-sm);
		background: rgba(var(--accent-rgb), 0.08);
		color: var(--text-primary);
		font-size: 0.72rem;
		line-height: 1.4;
	}

	.help-text,
	.load-error {
		margin: 0;
		font-size: 0.78rem;
		line-height: 1.5;
		color: var(--text-secondary);
	}

	.load-error {
		color: var(--color-error);
	}

	.change-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		white-space: normal;
	}
</style>
