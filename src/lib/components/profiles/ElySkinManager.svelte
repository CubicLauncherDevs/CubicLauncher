<script lang="ts">
	import { onMount } from "svelte";
	import { getElySkinProfile, openUrl } from "$lib/api/cubicApi";
	import { bumpAvatarVersion } from "$lib/state/avatarCache.svelte";
	import type { ElySkinProfile } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import Skin3dViewer from "./Skin3dViewer.svelte";

	let { uuid }: { uuid: string } = $props();
	let profile = $state<ElySkinProfile | null>(null);
	let loading = $state(false);
	let error = $state(false);
	let refreshOnReturn = false;
	let disposed = false;

	async function refresh(updateAvatar = true) {
		if (loading || disposed) return;
		loading = true;
		error = false;
		try {
			const next = await getElySkinProfile(uuid);
			if (disposed) return;
			profile = next;
			if (updateAvatar) bumpAvatarVersion(uuid);
		} catch {
			if (!disposed) error = true;
		} finally {
			if (!disposed) loading = false;
		}
	}

	async function changeSkin() {
		refreshOnReturn = true;
		await openUrl("https://ely.by/skins");
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

<section class="ely-skin-manager" aria-label={t("userMenu.elySkin.title")}>
	<div class="section-header">
		<h4>{t("userMenu.elySkin.title")}</h4>
		<button
			type="button"
			class="btn-secondary refresh-btn"
			onclick={() => refresh()}
			disabled={loading}
			title={t("userMenu.elySkin.refresh")}
			aria-label={t("userMenu.elySkin.refresh")}
		>
			<Icon name="ui:refresh" size={16} />
		</button>
	</div>
	{#if error}
		<p class="load-error" role="alert">{t("userMenu.elySkin.loadError")}</p>
	{/if}
	<div class="preview" aria-busy={loading}>
		{#if profile?.skin_url}
			<Skin3dViewer
				skinUrl={profile.skin_url}
				capeUrl={null}
				model={profile.model === "slim" ? "slim" : "default"}
				quality="high"
			/>
		{:else}
			<p class="preview-status" role="status">
				{loading
					? t("userMenu.skinCape.loading")
					: error
						? t("userMenu.elySkin.unavailable")
						: t("userMenu.elySkin.noSkin")}
			</p>
		{/if}
	</div>
	{#if profile?.skin_url}
		<span class="model-label">
			{profile.model === "slim"
				? t("userMenu.skinCape.slim")
				: t("userMenu.skinCape.classic")}
		</span>
	{/if}
	<p class="help-text">{t("userMenu.elySkin.description")}</p>
	<button type="button" class="btn-primary change-btn" onclick={changeSkin}>
		<Icon name="instance:external-link" size={16} />
		{t("userMenu.elySkin.change")}
	</button>
	<p class="help-text">{t("userMenu.elySkin.refreshHint")}</p>
</section>

<style>
	.ely-skin-manager {
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

	.help-text,
	.model-label,
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
