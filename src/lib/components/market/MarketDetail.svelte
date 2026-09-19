<script lang="ts">
	import { onDestroy } from "svelte";
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
	import MarketProjectInfo from "./MarketProjectInfo.svelte";
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
		icon?: string | null;
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
		localActionsDisabled?: boolean;
		localActionError?: string;
	}

	let {
		project,
		icon = project.icon,
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
		localActionsDisabled = false,
		localActionError,
	}: Props = $props();

	let installing = $state(false);
	let actionError = $state<string | null>(null);
	let iconError = $state(false);
	$effect(() => {
		void icon;
		iconError = false;
	});

	let modalOpen = $state(false);
	let resolvingDeps = $state(false);
	let depTree = $state<ResolvedDependency[]>([]);
	let depConflicts = $state<DependencyConflict[]>([]);
	let installedProjectIds = $state<Set<string>>(new Set());
	let modalError = $state<string | null>(null);
	let disposed = false;
	let downloading = false;
	let installGeneration = 0;

	function releaseDependencies() {
		depTree = [];
		depConflicts = [];
		installedProjectIds = new Set();
	}
	onDestroy(() => {
		disposed = true;
		installGeneration++;
		releaseDependencies();
	});

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
		if (disposed || !selectedVersion) return;
		const generation = ++installGeneration;

		installing = true;
		actionError = null;

		try {
			if (contentType !== "mods") {
				const single = await buildSingleDownload(
					project,
					selectedVersion,
				);
				if (disposed || generation !== installGeneration) return;
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
			if (disposed || generation !== installGeneration) return;
			depTree = result.tree;
			depConflicts = result.conflicts;
			installedProjectIds = result.installedProjectIds;
		} catch (e) {
			if (disposed || generation !== installGeneration) return;
			modalError = String(e ?? "Install failed");
			actionError = String(e ?? "Install failed");
		} finally {
			if (!disposed && generation === installGeneration) {
				resolvingDeps = false;
				installing = false;
			}
		}
	}

	async function handleConfirmInstall(queue: ModDownloadInfo[]) {
		if (disposed || downloading) return;
		downloading = true;
		installing = true;
		actionError = null;
		try {
			await onInstallQueue(queue);
			if (disposed) return;
			modalOpen = false;
			releaseDependencies();
		} catch (e) {
			if (disposed) return;
			actionError = String(e ?? "Install failed");
			modalError = String(e ?? "Install failed");
		} finally {
			downloading = false;
			if (!disposed) installing = false;
		}
	}

	function handleCancelInstall() {
		installGeneration++;
		resolvingDeps = false;
		installing = downloading;
		modalOpen = false;
		modalError = null;
		releaseDependencies();
	}

	const modrinthTypePath = $derived(
		contentType === "resourcepacks"
			? "resourcepack"
			: contentType === "shaderpacks"
				? "shader"
				: "mod",
	);
</script>

<div class="market-detail">
	<div class="market-detail-header">
		<button type="button" class="market-detail-close" onclick={onClose}>
			<Icon name="ui:chevron-left" size={16} />
			{t("market.detail.backToCatalog")}
		</button>
		{#if detail.loading}
			<span class="market-detail-loading" role="status">
				<Loading class="detail-version-spinner" />
				{t("market.detail.loadingDetails")}
			</span>
		{/if}
	</div>

	<div class="market-detail-scroll">
		<div class="market-detail-columns">
			<div class="market-detail-hero">
				<div class="market-detail-icon">
					{#if icon && !iconError}
						<img
							src={icon}
							alt={project.title}
							loading="lazy"
							decoding="async"
							onerror={() => (iconError = true)}
						/>
					{:else}
						<CubicLogo />
					{/if}
				</div>

				<div class="market-detail-identity">
					<h2 class="market-detail-title">{project.title}</h2>
					<p class="market-detail-author">
						{t("market.detail.by")}
						{project.author || t("market.detail.unknownAuthor")}
					</p>
				</div>
			</div>

			<aside
				class="market-detail-sidebar"
				aria-label={t("market.detail.projectInfo")}
			>
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
									>{t("market.detail.selectedVersion")}</span
								>
								<Dropdown
									value={selectedVersion?.id ?? ""}
									options={versionOptions}
									placeholder={t(
										"market.detail.selectVersion",
									)}
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
							{#if contentType === "mods"}
								<button
									type="button"
									class="market-detail-btn secondary"
									disabled={localActionsDisabled}
									onclick={onToggleEnabled}
								>
									{project.disabled
										? t("market.detail.enable")
										: t("market.detail.disable")}
								</button>
							{:else}
								<p class="market-detail-installed-label">
									{t("market.manage.packHint")}
								</p>
							{/if}
							<button
								type="button"
								class="market-detail-btn danger"
								disabled={localActionsDisabled}
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

				{#if actionError || (source === "local" && localActionError)}
					<p class="market-detail-action-error" role="alert">
						{actionError || localActionError}
					</p>
				{/if}

				<MarketProjectInfo {project} {detail} {contentType} />
			</aside>

			<div class="market-detail-content">
				{#if project.description}
					<p class="market-detail-description">
						{project.description}
					</p>
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
			</div>
		</div>
	</div>
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
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex-wrap: wrap;
		gap: 10px;
		padding: 12px var(--market-page-padding);
		background: var(--market-panel-bg);
		border-bottom: var(--border-width) solid var(--border);
		flex-shrink: 0;
	}

	.market-detail-close {
		padding: 7px 10px;
		font: inherit;
		font-size: 0.8rem;
		gap: 6px;
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			background-color 0.15s,
			color 0.15s;
	}

	.market-detail-close:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.market-detail-scroll {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--market-page-padding);
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.market-detail-hero {
		grid-column: 1;
		grid-row: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 18px;
	}

	.market-detail-identity {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.market-detail-columns {
		display: grid;
		grid-template-columns: minmax(0, 1fr) var(
				--market-detail-sidebar-width
			);
		grid-template-rows: auto 1fr;
		align-items: start;
		gap: 24px;
	}

	.market-detail-sidebar {
		grid-column: 2;
		grid-row: 1 / 3;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.market-detail-content {
		grid-column: 1;
		grid-row: 2;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 20px;
		overflow-wrap: anywhere;
	}

	.market-detail-icon {
		width: var(--market-detail-icon-size);
		height: var(--market-detail-icon-size);
		flex-shrink: 0;
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
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
		object-fit: contain;
	}

	.market-detail-title {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--text-primary);
		overflow-wrap: anywhere;
		margin: 0;
	}

	.market-detail-author {
		font-size: 0.85rem;
		color: var(--text-secondary);
		margin: 0;
	}

	.market-detail-stats {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(80px, 1fr));
		gap: 8px;
	}

	.market-detail-stat {
		padding-bottom: 14px;
		border-bottom: 1px solid var(--border);
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
		font-size: 1.15rem;
		font-variant-numeric: tabular-nums;
		font-weight: 700;
		color: var(--text-primary);
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
		background: var(--surface-input);
		border: 1px dashed var(--border);
		border-radius: var(--border-radius-sm);
	}

	:global(.detail-version-spinner) {
		width: 14px;
		height: 14px;
	}

	.market-detail-actions {
		display: flex;
		flex-wrap: wrap;
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
		transition:
			background-color 0.15s,
			color 0.15s;
		font-family: inherit;
	}

	.market-detail-btn.primary {
		background: var(--accent);
		color: var(--accent-text);
	}

	.market-detail-btn.primary:hover:not(:disabled) {
		background: var(--accent-hover);
	}

	.market-detail-btn.secondary {
		background: var(--surface-input);
		color: var(--text-primary);
		border: 1px solid var(--border);
	}

	.market-detail-btn.secondary:hover {
		background: var(--surface-hover);
	}

	.market-detail-btn.danger {
		background: color-mix(in srgb, var(--color-error) 12%, transparent);
		color: var(--color-error);
		border: var(--border-width) solid
			color-mix(in srgb, var(--color-error) 30%, transparent);
	}

	.market-detail-btn.danger:hover {
		background: color-mix(in srgb, var(--color-error) 20%, transparent);
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
		background: color-mix(in srgb, var(--color-success) 8%, transparent);
		border: var(--border-width) solid
			color-mix(in srgb, var(--color-success) 25%, transparent);
		border-radius: var(--border-radius-sm);
		letter-spacing: 0.3px;
	}

	.market-detail-action-error {
		color: var(--color-error);
		font-size: 0.75rem;
		margin: 0;
		text-align: center;
		overflow-wrap: anywhere;
	}

	.market-detail-description {
		font-size: 0.85rem;
		line-height: 1.55;
		color: var(--text-tertiary, var(--text-secondary));
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
		gap: 16px;
		padding-top: 20px;
		border-top: 1px solid var(--border);
	}

	.market-detail-loading {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--text-secondary);
		font-size: 0.75rem;
	}

	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 3px;
	}

	@container market (max-width: 700px) {
		.market-detail-header {
			padding: 10px var(--market-page-padding-compact);
		}
		.market-detail-scroll {
			padding: 16px var(--market-page-padding-compact);
			gap: 18px;
		}
		.market-detail-columns {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: auto;
			gap: 20px;
		}
		.market-detail-hero,
		.market-detail-sidebar,
		.market-detail-content {
			grid-column: auto;
			grid-row: auto;
		}
		.market-detail-icon {
			width: calc(var(--market-detail-icon-size) * 0.75);
			height: calc(var(--market-detail-icon-size) * 0.75);
		}
		.market-detail-title {
			font-size: 1.2rem;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		button {
			transition: none;
		}
	}
</style>
