<script lang="ts">
	import { t } from "$lib/i18n";
	import { openUrl } from "$lib/api/cubicApi";
	import Loading from "$lib/icons/Loading.svelte";
	import Dropdown from "$lib/components/layout/Dropdown.svelte";
	import { renderMarkdown } from "$lib/util/markdown";
	import type { MarketDetailState } from "$lib/state/marketState.svelte";
	import type {
		MarketProject,
		MarketVersion,
		ContentType,
	} from "$lib/types/market";
	import type {
		ModrinthProjectFull,
		CurseForgeProject,
	} from "$lib/types/types";

	interface Props {
		project: MarketProject;
		contentType: ContentType;
		detail: MarketDetailState;
		selectedVersion: MarketVersion | null;
		isVersionCompatible: (version: MarketVersion) => boolean;
		onVersionSelect: (version: MarketVersion) => void;
		onInstall: () => void;
		onUninstall: () => void;
		onToggleEnabled: () => void;
		onClose: () => void;
	}

	let {
		project,
		contentType = "mods",
		detail,
		selectedVersion,
		isVersionCompatible,
		onVersionSelect,
		onInstall,
		onUninstall,
		onToggleEnabled,
		onClose,
	}: Props = $props();

	let installing = $state(false);
	let actionError = $state<string | null>(null);

	const readmeHtml = $derived(
		project.source !== "curseforge" &&
			(detail.fullProject as ModrinthProjectFull | undefined)?.body
			? renderMarkdown((detail.fullProject as ModrinthProjectFull).body!)
			: "",
	);

	const versionOptions = $derived(
		detail.versions.map((v) => ({
			value: v.id,
			label: `${v.versionNumber} — ${v.name}`,
			subtitle: isVersionCompatible(v)
				? "✓ Compatible"
				: v.gameVersions.slice(0, 2).join(", "),
		})),
	);

	function formatNumber(num: number | undefined | null): string {
		if (num == null) return "—";
		if (num >= 1_000_000) return (num / 1_000_000).toFixed(1) + "M";
		if (num >= 1_000) return (num / 1_000).toFixed(1) + "K";
		return num.toString();
	}

	async function handleInstall() {
		installing = true;
		actionError = null;
		try {
			await onInstall();
		} catch (e) {
			actionError = String(e ?? "Install failed");
		} finally {
			installing = false;
		}
	}

	const modrinthTypePath = $derived(
		contentType === "resourcepacks"
			? "resourcepack"
			: contentType === "shaderpacks"
				? "shader"
				: "mod",
	);

	function openProjectUrl() {
		if (project.source === "curseforge") {
			const slug =
				(detail.fullProject as CurseForgeProject | undefined)?.slug ??
				project.curseforge?.slug;
			if (slug) {
				openUrl(`https://curseforge.com/minecraft/mc-mods/${slug}`);
			}
			return;
		}
		const slug = detail.fullProject?.slug ?? project.modrinth?.slug;
		if (slug) {
			openUrl(`https://modrinth.com/${modrinthTypePath}/${slug}`);
		}
	}
</script>

<div class="market-detail">
	<div class="market-detail-header">
		<button
			type="button"
			class="market-detail-close"
			onclick={onClose}
			aria-label={t("market.detail.close")}
		>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.5"
			>
				<path d="M18 6 6 18M6 6l12 12" />
			</svg>
		</button>
	</div>

	<div class="market-detail-scroll">
		<div class="market-detail-icon">
			{#if project.icon}
				<img src={project.icon} alt={project.title} loading="lazy" />
			{:else}
				<span>📦</span>
			{/if}
		</div>

		<h3 class="market-detail-title">{project.title}</h3>
		<p class="market-detail-author">
			{t("market.detail.by")}
			{project.author || t("market.detail.unknownAuthor")}
		</p>

		{#if project.source !== "local"}
			<div class="market-detail-stats">
				<div class="market-detail-stat">
					<span class="market-detail-stat-label"
						>{t("market.detail.downloads")}</span
					>
					<span class="market-detail-stat-value"
						>{formatNumber(project.downloadCount)}</span
					>
				</div>
			</div>
		{/if}

		{#if project.source !== "curseforge" && (detail.fullProject as ModrinthProjectFull)?.categories?.length}
			<div class="market-detail-tags">
				{#each (detail.fullProject as ModrinthProjectFull).categories as category (category)}
					<span class="market-detail-tag">{category}</span>
				{/each}
			</div>
		{/if}

		<div class="market-detail-version">
			{#if detail.loading || detail.versions.length === 0}
				<span class="market-detail-version-loading">
					{#if detail.loading}
						<Loading class="detail-version-spinner" />
					{/if}
					{detail.loading
						? t("market.detail.loadingVersions")
						: t("market.detail.noVersions")}
				</span>
			{:else}
				<div class="market-detail-version-row">
					<span class="market-detail-version-label"
						>{t("market.detail.version")}</span
					>
					<Dropdown
						value={selectedVersion?.id ?? ""}
						options={versionOptions}
						placeholder={t("market.detail.selectVersion")}
						onchange={(value) => {
							const version = detail.versions.find(
								(v) => v.id === value,
							);
							if (version) onVersionSelect(version);
						}}
					/>
				</div>
			{/if}
		</div>

		<div class="market-detail-actions">
			{#if project.installed}
				<button
					type="button"
					class="market-detail-btn secondary"
					onclick={onToggleEnabled}
				>
					{project.disabled
						? t("market.detail.enable")
						: t("market.detail.disable")}
				</button>
				<button
					type="button"
					class="market-detail-btn danger"
					onclick={onUninstall}
				>
					{t("market.detail.uninstall")}
				</button>
			{:else if selectedVersion}
				<button
					type="button"
					class="market-detail-btn primary"
					disabled={installing || !selectedVersion.primaryFileUrl}
					onclick={handleInstall}
				>
					{#if installing}
						<Loading class="detail-version-spinner" />
					{/if}
					{t("market.detail.install")}
				</button>
			{/if}
		</div>

		{#if actionError}
			<p class="market-detail-action-error">{actionError}</p>
		{/if}

		{#if project.description}
			<p class="market-detail-description">{project.description}</p>
		{/if}

		{#if readmeHtml}
			<div class="market-detail-readme">
				<h4 class="market-detail-section-title">
					{t("market.detail.readme")}
				</h4>
				<div
					class="markdown-body"
					role="presentation"
					onclick={(e) => {
						const a = (e.target as HTMLElement).closest("a");
						if (a?.href && !a.href.startsWith("#")) {
							e.preventDefault();
							openUrl(a.href);
						}
					}}
				>
					{@html readmeHtml}
				</div>
			</div>
		{/if}

		{#if project.source !== "curseforge" && (detail.fullProject as ModrinthProjectFull | undefined)?.gallery?.length}
			<div class="market-detail-gallery">
				<h4 class="market-detail-section-title">
					{t("market.detail.gallery")}
				</h4>
				<div class="gallery-grid">
					{#each (detail.fullProject as ModrinthProjectFull).gallery as image, i (i)}
						<img src={image.url} alt="Gallery" loading="lazy" />
					{/each}
				</div>
			</div>
		{/if}

		{#if project.source !== "local"}
			<button
				type="button"
				class="market-detail-open-link"
				onclick={openProjectUrl}
			>
				{project.source === "curseforge"
					? t("market.detail.openOnCurseForge")
					: t("market.detail.openOnModrinth")}
			</button>
		{/if}
	</div>

	{#if detail.loading && !detail.fullProject && detail.versions.length === 0}
		<div class="market-detail-loading">
			<span class="spinner"></span>
		</div>
	{/if}
</div>

<style>
	.market-detail {
		position: relative;
		height: 100%;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.market-detail-header {
		position: absolute;
		top: 14px;
		right: 14px;
		z-index: 2;
	}

	.market-detail-close {
		width: 28px;
		height: 28px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s;
	}

	.market-detail-close:hover {
		background: rgba(255, 255, 255, 0.1);
		color: var(--text-primary);
	}

	.market-detail-scroll {
		flex: 1;
		overflow-y: auto;
		padding: 20px 16px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.market-detail-icon {
		width: 80px;
		height: 80px;
		margin: 8px auto 0;
		border-radius: var(--border-radius-sm);
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 2rem;
	}

	.market-detail-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		image-rendering: pixelated;
	}

	.market-detail-title {
		font-size: 1rem;
		font-weight: 700;
		color: var(--text-primary);
		text-align: center;
		margin: 0;
	}

	.market-detail-author {
		font-size: 0.75rem;
		color: var(--text-secondary);
		text-align: center;
		margin: 0;
	}

	.market-detail-stats {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(80px, 1fr));
		gap: 8px;
	}

	.market-detail-stat {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 8px 6px;
		text-align: center;
	}

	.market-detail-stat-label {
		display: block;
		font-size: 0.6rem;
		text-transform: uppercase;
		letter-spacing: 0.8px;
		color: var(--text-secondary);
		margin-bottom: 4px;
	}

	.market-detail-stat-value {
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--text-primary);
	}

	.market-detail-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.market-detail-tag {
		font-size: 0.68rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border);
		padding: 2px 8px;
		border-radius: 20px;
		color: var(--text-secondary);
		text-transform: capitalize;
	}

	.market-detail-version {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.market-detail-version-row {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.market-detail-version-label {
		font-size: 0.65rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.8px;
		color: var(--text-secondary);
	}

	.market-detail-version-loading {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		font-size: 0.78rem;
		color: var(--text-secondary);
		padding: 10px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px dashed var(--border);
		border-radius: var(--border-radius-sm);
	}

	:global(.detail-version-spinner) {
		width: 14px;
		height: 14px;
	}

	.market-detail-actions {
		display: flex;
		gap: 8px;
	}

	.market-detail-btn {
		flex: 1;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 8px 12px;
		border: none;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.78rem;
		font-weight: 700;
		letter-spacing: 0.3px;
		transition: all 0.15s;
		font-family: inherit;
	}

	.market-detail-btn.primary {
		background: var(--accent);
		color: var(--bg-main);
	}

	.market-detail-btn.primary:hover:not(:disabled) {
		filter: brightness(0.9);
	}

	.market-detail-btn.secondary {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary);
		border: 1px solid var(--border);
	}

	.market-detail-btn.secondary:hover {
		background: rgba(255, 255, 255, 0.1);
	}

	.market-detail-btn.danger {
		background: rgba(255, 68, 68, 0.12);
		color: #ff6b6b;
		border: 1px solid rgba(255, 68, 68, 0.3);
	}

	.market-detail-btn.danger:hover {
		background: rgba(255, 68, 68, 0.2);
	}

	.market-detail-btn:disabled {
		opacity: 0.5;
		cursor: wait;
	}

	.market-detail-action-error {
		color: #f87171;
		font-size: 0.75rem;
		margin: 0;
		text-align: center;
	}

	.market-detail-description {
		font-size: 0.8rem;
		line-height: 1.55;
		color: var(--text-secondary);
		margin: 0;
	}

	.market-detail-section-title {
		font-size: 0.72rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.8px;
		color: var(--text-secondary);
		margin: 0;
	}

	.market-detail-readme {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	:global(.markdown-body) {
		font-size: 0.82rem;
		line-height: 1.6;
		color: var(--text-secondary);
		word-break: break-word;
	}

	:global(.markdown-body h1),
	:global(.markdown-body h2),
	:global(.markdown-body h3),
	:global(.markdown-body h4) {
		color: var(--text-primary);
		margin: 1.2em 0 0.6em;
	}

	:global(.markdown-body p) {
		margin: 0.6em 0;
	}

	:global(.markdown-body a) {
		color: var(--accent);
		text-decoration: none;
	}

	:global(.markdown-body a:hover) {
		text-decoration: underline;
	}

	:global(.markdown-body img) {
		max-width: 100%;
		height: auto;
		border-radius: var(--border-radius-sm);
	}

	:global(.markdown-body code) {
		background: rgba(255, 255, 255, 0.06);
		padding: 2px 5px;
		border-radius: 4px;
		font-size: 0.78rem;
	}

	:global(.markdown-body pre) {
		background: rgba(255, 255, 255, 0.04);
		padding: 10px;
		border-radius: var(--border-radius-sm);
		overflow-x: auto;
	}

	:global(.markdown-body ul),
	:global(.markdown-body ol) {
		padding-left: 20px;
	}

	:global(.markdown-body table) {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.75rem;
	}

	:global(.markdown-body th),
	:global(.markdown-body td) {
		border: 1px solid var(--border);
		padding: 6px 8px;
		text-align: left;
	}

	:global(.markdown-body th) {
		background: rgba(255, 255, 255, 0.04);
	}

	.market-detail-gallery {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.gallery-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
		gap: 8px;
	}

	.gallery-grid img {
		width: 100%;
		aspect-ratio: 16 / 9;
		object-fit: cover;
		border-radius: var(--border-radius-sm);
		border: 1px solid var(--border);
	}

	.market-detail-open-link {
		padding: 8px;
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-secondary);
		font-size: 0.75rem;
		font-weight: 700;
		cursor: pointer;
		transition: all 0.15s;
		font-family: inherit;
	}

	.market-detail-open-link:hover {
		background: rgba(255, 255, 255, 0.05);
		color: var(--text-primary);
	}

	.market-detail-loading {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(var(--bg-main), 0.8);
		backdrop-filter: blur(2px);
	}

	.spinner {
		width: 28px;
		height: 28px;
		border: 3px solid var(--border);
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
