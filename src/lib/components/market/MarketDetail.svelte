<script lang="ts">
	import { t } from "$lib/i18n";
	import {
		openUrl,
		getCurseForgeFileDownloadUrl,
		type ModDownloadInfo,
	} from "$lib/api/cubicApi";
	import Icon from "$lib/icons/Icon.svelte";
	import Loading from "$lib/icons/Loading.svelte";
	import CubicLogo from "./CubicLogo.svelte";
	import Dropdown from "$lib/components/layout/Dropdown.svelte";
	import MarkdownRenderer from "$lib/components/ui/MarkdownRenderer.svelte";
	import HtmlRenderer from "$lib/components/ui/HtmlRenderer.svelte";
	import MarketDependenciesModal from "./MarketDependenciesModal.svelte";
	import type { MarketDetailState } from "$lib/state/marketState.svelte";
	import type {
		MarketProject,
		MarketVersion,
		ContentType,
		MarketSource,
	} from "$lib/types/market";
	import type { ModrinthProjectFull } from "$lib/types/types";
	import type {
		DependencyConflict,
		ResolvedDependency,
	} from "$lib/types/dependency";

	interface Props {
		project: MarketProject;
		source: MarketSource;
		contentType: ContentType;
		detail: MarketDetailState;
		selectedVersion: MarketVersion | null;
		isVersionCompatible: (version: MarketVersion) => boolean;
		onVersionSelect: (version: MarketVersion) => void;
		onPrepareInstall: () => Promise<{
			tree: ResolvedDependency[];
			conflicts: DependencyConflict[];
			installedProjectIds: Set<string>;
		}>;
		onInstallQueue: (queue: ModDownloadInfo[]) => Promise<void>;
		onUninstall: () => void;
		onToggleEnabled: () => void;
		onClose: () => void;
	}

	let {
		project,
		source = "modrinth",
		contentType = "mods",
		detail,
		selectedVersion,
		isVersionCompatible,
		onVersionSelect,
		onPrepareInstall,
		onInstallQueue,
		onUninstall,
		onToggleEnabled,
		onClose,
	}: Props = $props();

	let installing = $state(false);
	let actionError = $state<string | null>(null);
	let iconError = $state(false);

	let modalOpen = $state(false);
	let resolvingDeps = $state(false);
	let depTree = $state<ResolvedDependency[]>([]);
	let depConflicts = $state<DependencyConflict[]>([]);
	let installedProjectIds = $state<Set<string>>(new Set());
	let modalError = $state<string | null>(null);

	const bodySource = $derived(
		project.source !== "curseforge"
			? ((detail.fullProject as ModrinthProjectFull | undefined)?.body ??
					"")
			: "",
	);

	const curseforgeBodySource = $derived(
		project.source === "curseforge"
			? (detail.curseforgeDescription ?? "")
			: "",
	);

	const readmeBaseUrl = $derived.by(() => {
		if (project.source === "curseforge") return undefined;
		const slug =
			(detail.fullProject as ModrinthProjectFull | undefined)?.slug ??
			project.slug;
		if (!slug) return undefined;
		return `https://modrinth.com/${modrinthTypePath}/${slug}`;
	});

	const versionOptions = $derived.by(() => {
		const compatible = detail.versions.filter((v) =>
			isVersionCompatible(v),
		);
		const versionsToShow =
			compatible.length > 0 ? compatible : detail.versions;
		return versionsToShow.map((v) => ({
			value: v.id,
			label: `${v.versionNumber} — ${v.name}`,
			subtitle:
				compatible.length > 0
					? "✓ Compatible"
					: v.gameVersions.slice(0, 2).join(", "),
		}));
	});

	function formatNumber(num: number | undefined | null): string {
		if (num == null) return "—";
		if (num >= 1_000_000) return (num / 1_000_000).toFixed(1) + "M";
		if (num >= 1_000) return (num / 1_000).toFixed(1) + "K";
		return num.toString();
	}

	async function buildSingleDownload(
		project: MarketProject,
		version: MarketVersion,
	): Promise<ModDownloadInfo | null> {
		let url = version.primaryFileUrl;
		const projectId =
			project.source === "curseforge"
				? (project.curseforgeProjectId ?? project.id)
				: (project.modrinthProjectId ?? project.id);

		if (project.source === "curseforge" && !url) {
			url =
				(await getCurseForgeFileDownloadUrl(
					Number(projectId),
					Number(version.id),
				)) ?? "";
		}

		if (!url) return null;

		return {
			url,
			filename: version.primaryFileName,
			project_id: projectId,
			version_id: version.id,
		};
	}

	async function handleInstall() {
		if (!selectedVersion) return;

		installing = true;
		actionError = null;

		try {
			if (contentType !== "mods") {
				const single = await buildSingleDownload(
					project,
					selectedVersion,
				);
				if (!single) {
					throw new Error("No download URL available");
				}
				await onInstallQueue([single]);
				return;
			}

			modalOpen = true;
			resolvingDeps = true;
			modalError = null;
			depTree = [];
			depConflicts = [];
			installedProjectIds = new Set();

			const result = await onPrepareInstall();
			depTree = result.tree;
			depConflicts = result.conflicts;
			installedProjectIds = result.installedProjectIds;
		} catch (e) {
			modalError = String(e ?? "Install failed");
			actionError = String(e ?? "Install failed");
		} finally {
			resolvingDeps = false;
			installing = false;
		}
	}

	async function handleConfirmInstall(queue: ModDownloadInfo[]) {
		installing = true;
		actionError = null;
		try {
			await onInstallQueue(queue);
			modalOpen = false;
		} catch (e) {
			actionError = String(e ?? "Install failed");
			modalError = String(e ?? "Install failed");
		} finally {
			installing = false;
		}
	}

	function handleCancelInstall() {
		modalOpen = false;
		modalError = null;
	}

	const modrinthTypePath = $derived(
		contentType === "resourcepacks"
			? "resourcepack"
			: contentType === "shaderpacks"
				? "shader"
				: "mod",
	);

	function openProjectUrl() {
		const slug = detail.fullProject?.slug ?? project.slug;
		if (!slug) return;

		if (project.source === "curseforge") {
			openUrl(`https://curseforge.com/minecraft/mc-mods/${slug}`);
			return;
		}
		openUrl(`https://modrinth.com/${modrinthTypePath}/${slug}`);
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
			<Icon src="/images/icons/ui/close.svg" size={14} />
		</button>
	</div>

	<div class="market-detail-scroll">
		<div class="market-detail-icon">
			{#if project.icon && !iconError}
				<img
					src={project.icon}
					alt={project.title}
					loading="lazy"
					decoding="async"
					onerror={() => (iconError = true)}
				/>
			{:else}
				<CubicLogo />
			{/if}
		</div>

		<h3 class="market-detail-title">{project.title}</h3>
		<p class="market-detail-author">
			{t("market.detail.by")}
			{project.author || t("market.detail.unknownAuthor")}
		</p>

		{#if source !== "local"}
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

		{#if source !== "local"}
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
		{/if}

		<div class="market-detail-actions">
			{#if project.installed}
				{#if source === "local"}
					<span class="market-detail-installed-label">
						{project.installed.version
							? `v${project.installed.version}`
							: t("market.detail.installedLabel")}
					</span>
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
				{:else}
					<span class="market-detail-installed-label">
						{t("market.detail.installedLabel")}
					</span>
				{/if}
			{:else if selectedVersion}
				<button
					type="button"
					class="market-detail-btn primary"
					disabled={installing ||
						(contentType !== "mods" &&
							!selectedVersion.primaryFileUrl)}
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

		{#if bodySource || curseforgeBodySource}
			<div class="market-detail-readme">
				<h4 class="market-detail-section-title">
					{t("market.detail.readme")}
				</h4>
				{#if project.source === "curseforge"}
					<HtmlRenderer
						source={curseforgeBodySource}
						onLinkClick={openUrl}
					/>
				{:else}
					<MarkdownRenderer
						source={bodySource}
						baseUrl={readmeBaseUrl}
						onLinkClick={openUrl}
					/>
				{/if}
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

<MarketDependenciesModal
	bind:open={modalOpen}
	projectTitle={project.title}
	tree={depTree}
	conflicts={depConflicts}
	{installedProjectIds}
	resolving={resolvingDeps}
	downloading={installing}
	error={modalError}
	onConfirm={handleConfirmInstall}
	onCancel={handleCancelInstall}
	onclose={handleCancelInstall}
/>

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
		background: rgba(var(--color-error-rgb), 0.12);
		color: var(--color-error);
		border: 1px solid rgba(var(--color-error-rgb), 0.3);
	}

	.market-detail-btn.danger:hover {
		background: rgba(var(--color-error-rgb), 0.2);
	}

	.market-detail-btn:disabled {
		opacity: 0.5;
		cursor: wait;
	}

	.market-detail-installed-label {
		flex: 1;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 8px 12px;
		font-size: 0.78rem;
		font-weight: 700;
		color: var(--color-success);
		background: rgba(var(--color-success-rgb), 0.08);
		border: 1px solid rgba(var(--color-success-rgb), 0.25);
		border-radius: var(--border-radius-sm);
		letter-spacing: 0.3px;
	}

	.market-detail-action-error {
		color: var(--color-error);
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
