<script lang="ts">
	import { t } from "$lib/i18n";
	import Loading from "$lib/icons/Loading.svelte";
	import { SvelteSet } from "svelte/reactivity";
	import type { ModDownloadInfo } from "$lib/api/cubicApi";
	import type {
		DependencyConflict,
		ResolvedDependency,
	} from "$lib/types/dependency";

	let {
		resolvingDeps = false,
		downloading = false,
		downloadQueue = [] as ModDownloadInfo[],
		dependencyTree = [] as ResolvedDependency[],
		conflicts = [] as DependencyConflict[],
		installedProjectIds = new Set<string>(),
		onBack,
		onConfirmDownload,
		onQueueChange,
	}: {
		resolvingDeps: boolean;
		downloading: boolean;
		downloadQueue: ModDownloadInfo[];
		dependencyTree: ResolvedDependency[];
		conflicts: DependencyConflict[];
		installedProjectIds?: Set<string>;
		onBack: () => void;
		onConfirmDownload: () => Promise<void>;
		onQueueChange?: (queue: ModDownloadInfo[]) => void;
	} = $props();

	let selectedOptionalIds = new SvelteSet<string>();

	function isInstalled(dep: ResolvedDependency): boolean {
		return installedProjectIds.has(dep.project_id);
	}

	function badgeLabel(kind: ResolvedDependency["kind"]): string {
		const key = `instanceView.downloadMods.dependencyKind.${kind}`;
		const value = t(key as never);
		return value === key ? kind : value;
	}

	function computeQueue(deps: ResolvedDependency[]): ModDownloadInfo[] {
		const queue: ModDownloadInfo[] = [];
		const seen = new SvelteSet<string>();

		function walk(items: ResolvedDependency[]) {
			for (const dep of items) {
				if (dep.kind === "incompatible") continue;
				if (isInstalled(dep)) continue;
				if (
					dep.kind === "optional" &&
					!selectedOptionalIds.has(dep.project_id)
				) {
					continue;
				}
				if (dep.download_url && dep.filename) {
					const key = dep.filename.toLowerCase();
					if (!seen.has(key)) {
						seen.add(key);
						queue.push({
							url: dep.download_url,
							filename: dep.filename,
							projectTitle: dep.title,
							iconUrl: dep.icon_url ?? undefined,
						});
					}
				}
				walk(dep.children);
			}
		}

		walk(deps);
		return queue;
	}

	$effect(() => {
		if (dependencyTree.length > 0) {
			void selectedOptionalIds;
			onQueueChange?.(computeQueue(dependencyTree));
		}
	});

	function toggleOptional(projectId: string) {
		if (selectedOptionalIds.has(projectId)) {
			selectedOptionalIds.delete(projectId);
		} else {
			selectedOptionalIds.add(projectId);
		}
	}

	function expandAllOptionals() {
		for (const dep of dependencyTree) {
			addOptionalsRecursively(dep);
		}
	}

	function addOptionalsRecursively(dep: ResolvedDependency) {
		if (dep.kind === "optional") {
			selectedOptionalIds.add(dep.project_id);
		}
		for (const child of dep.children) {
			addOptionalsRecursively(child);
		}
	}

	function collapseAllOptionals() {
		selectedOptionalIds.clear();
	}

	async function handleConfirm() {
		if (onQueueChange) {
			onQueueChange(computeQueue(dependencyTree));
		}
		await onConfirmDownload();
	}

	function hasOptionalRecursively(dep: ResolvedDependency): boolean {
		if (dep.kind === "optional") return true;
		return dep.children.some(hasOptionalRecursively);
	}
</script>

<div class="dm-review">
	<div class="dm-review-header">
		<div>
			<span class="dm-section-label"
				>{t("instanceView.downloadMods.sectionLabel")}</span
			>
			<h2 class="dm-review-title">
				{t("instanceView.downloadMods.reviewTitle")}
			</h2>
		</div>
		<button
			type="button"
			class="dm-back-btn"
			onclick={onBack}
			disabled={downloading}
		>
			{t("instanceView.downloadMods.back")}
		</button>
	</div>

	<div class="dm-review-body">
		{#if resolvingDeps}
			<div class="dm-center-state">
				<Loading />
				<p>{t("instanceView.downloadMods.resolvingDeps")}</p>
			</div>
		{:else if dependencyTree.length === 0}
			<div class="dm-center-state">
				<p>{t("instanceView.downloadMods.allInstalled")}</p>
				<span style="font-size:0.75rem; opacity:0.5;"
					>{t("instanceView.downloadMods.allInstalledSub")}</span
				>
			</div>
		{:else}
			{#if conflicts.length > 0}
				<div class="dm-conflicts-box">
					<strong
						>{t("instanceView.downloadMods.conflictsTitle")}</strong
					>
					<ul>
						{#each conflicts as conflict (conflict.project_id)}
							<li>
								{conflict.project_id}: {conflict.requested_versions
									.map((v) => v.version_id)
									.join(", ")}
							</li>
						{/each}
					</ul>
				</div>
			{/if}

			<div class="dm-queue-box">
				<div class="dm-queue-toolbar">
					<p class="dm-queue-subtitle">
						{downloadQueue.length === 1
							? t(
									"instanceView.downloadMods.filesToDownload_one",
									{
										count: downloadQueue.length,
									},
								)
							: t(
									"instanceView.downloadMods.filesToDownload_other",
									{
										count: downloadQueue.length,
									},
								)}
					</p>
					{#if dependencyTree.some((d) => hasOptionalRecursively(d))}
						<div class="dm-optional-actions">
							<button type="button" onclick={expandAllOptionals}>
								{t(
									"instanceView.downloadMods.selectAllOptional",
								)}
							</button>
							<button
								type="button"
								onclick={collapseAllOptionals}
							>
								{t(
									"instanceView.downloadMods.deselectAllOptional",
								)}
							</button>
						</div>
					{/if}
				</div>

				<div class="dm-dep-list">
					{#each dependencyTree as dep (dep.project_id)}
						{@render dependencyNode(dep)}
					{/each}
				</div>
			</div>
		{/if}
	</div>

	{#if !resolvingDeps && dependencyTree.length > 0}
		<div class="dm-review-footer">
			<span class="dm-review-count">
				<strong>{downloadQueue.length}</strong>
				{downloadQueue.length !== 1
					? t("instanceView.downloadMods.file_other")
					: t("instanceView.downloadMods.file_one")}
			</span>
			<button
				type="button"
				class="dm-primary-btn"
				onclick={handleConfirm}
				disabled={downloading || downloadQueue.length === 0}
			>
				{#if downloading}
					<Loading />
					{t("instanceView.downloadMods.downloading")}
				{:else}
					{t("instanceView.downloadMods.confirmDownload")}
				{/if}
			</button>
		</div>
	{/if}
</div>

{#snippet dependencyNode(dep: ResolvedDependency)}
	{@const isOptional = dep.kind === "optional"}
	{@const isIncompatible = dep.kind === "incompatible"}
	{@const installed = isInstalled(dep)}
	{@const hasOptional = hasOptionalRecursively(dep)}

	<div
		class="dm-dep-node"
		class:optional={isOptional}
		class:incompatible={isIncompatible}
	>
		<div
			class="dm-dep-row"
			style:padding-left="{dep.depth * 14}px"
			class:has-optional-children={hasOptional}
		>
			<div class="dm-dep-icon">
				{#if dep.icon_url}
					<img src={dep.icon_url} alt="" />
				{:else}
					<span>🧩</span>
				{/if}
			</div>
			<div class="dm-dep-info">
				<span class="dm-dep-title">{dep.title}</span>
				{#if dep.filename}
					<span class="dm-dep-filename">{dep.filename}</span>
				{/if}
			</div>
			<span class="dm-dep-badge dm-dep-badge-{dep.kind}">
				{badgeLabel(dep.kind)}
			</span>
			{#if installed}
				<span class="dm-dep-badge dm-dep-badge-installed">
					{t("instanceView.downloadMods.installed")}
				</span>
			{:else if isOptional}
				<label class="dm-optional-toggle">
					<input
						type="checkbox"
						checked={selectedOptionalIds.has(dep.project_id)}
						onchange={() => toggleOptional(dep.project_id)}
					/>
					{t("instanceView.downloadMods.include")}
				</label>
			{/if}
		</div>
		{#if dep.children.length > 0}
			<div class="dm-dep-children">
				{#each dep.children as child (child.project_id)}
					{@render dependencyNode(child)}
				{/each}
			</div>
		{/if}
	</div>
{/snippet}

<style>
	.dm-review {
		display: flex;
		flex-direction: column;
		height: 100%;
		padding: 28px 32px;
	}
	.dm-review-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 24px;
		padding-bottom: 16px;
		border-bottom: 1px solid var(--border);
	}
	.dm-review-title {
		font-size: 1.3rem;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0;
	}
	.dm-review-body {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.dm-section-label {
		font-size: 0.62rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1.5px;
		color: var(--text-secondary);
		margin-bottom: 8px;
		display: block;
	}

	.dm-conflicts-box {
		background: rgba(var(--color-error-rgb), 0.08);
		border: 1px solid rgba(var(--color-error-rgb), 0.25);
		border-radius: var(--border-radius-sm);
		padding: 12px 14px;
		margin-bottom: 16px;
		color: var(--color-error);
		font-size: 0.78rem;
	}
	.dm-conflicts-box strong {
		display: block;
		margin-bottom: 6px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		font-size: 0.65rem;
	}
	.dm-conflicts-box ul {
		margin: 0;
		padding-left: 16px;
	}
	.dm-conflicts-box li {
		margin-bottom: 2px;
	}

	.dm-queue-box {
		flex: 1;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 16px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.dm-queue-toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 14px;
		gap: 12px;
		flex-wrap: wrap;
	}
	.dm-queue-subtitle {
		font-size: 0.78rem;
		color: var(--text-secondary);
		margin: 0;
	}
	.dm-optional-actions {
		display: flex;
		gap: 8px;
	}
	.dm-optional-actions button {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 4px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.68rem;
		cursor: pointer;
		transition: all 0.15s;
	}
	.dm-optional-actions button:hover {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}

	.dm-dep-list {
		flex: 1;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.dm-dep-node {
		display: flex;
		flex-direction: column;
	}
	.dm-dep-node.incompatible {
		opacity: 0.6;
	}
	.dm-dep-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		min-height: 42px;
	}
	.dm-dep-row:hover {
		background: rgba(255, 255, 255, 0.04);
	}

	.dm-dep-icon {
		width: 22px;
		height: 22px;
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.9rem;
	}
	.dm-dep-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.dm-dep-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
		gap: 1px;
	}
	.dm-dep-title {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dm-dep-filename {
		font-size: 0.72rem;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dm-dep-badge {
		font-size: 0.58rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.4px;
		padding: 2px 6px;
		border-radius: 4px;
		flex-shrink: 0;
	}
	.dm-dep-badge-required {
		background: rgba(var(--color-success-rgb), 0.12);
		color: var(--color-success);
	}
	.dm-dep-badge-embedded {
		background: rgba(var(--color-info-rgb, 59 130 246), 0.12);
		color: var(--color-info, #3b82f6);
	}
	.dm-dep-badge-optional {
		background: rgba(var(--color-warning-rgb), 0.12);
		color: var(--color-warning);
	}
	.dm-dep-badge-incompatible {
		background: rgba(var(--color-error-rgb), 0.12);
		color: var(--color-error);
	}
	.dm-dep-badge-installed {
		background: rgba(var(--color-success-rgb), 0.12);
		color: var(--color-success);
	}
	.dm-optional-toggle {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 0.7rem;
		color: var(--text-secondary);
		cursor: pointer;
		flex-shrink: 0;
	}
	.dm-optional-toggle input {
		cursor: pointer;
	}
	.dm-dep-children {
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin-top: 2px;
	}

	.dm-review-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 16px;
		padding: 14px 16px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}
	.dm-review-count {
		font-size: 0.85rem;
		color: var(--text-secondary);
	}
	.dm-review-count strong {
		color: var(--text-primary);
		font-size: 1.1rem;
	}
	.dm-primary-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 8px 18px;
		background: var(--accent);
		color: var(--bg-main);
		border: none;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.82rem;
		font-weight: 700;
		letter-spacing: 0.3px;
		transition: all 0.15s;
	}
	.dm-primary-btn:hover:not(:disabled) {
		filter: brightness(0.9);
	}
	.dm-primary-btn:disabled {
		opacity: 0.35;
		cursor: not-allowed;
	}

	.dm-back-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 7px 14px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.82rem;
		transition: all 0.15s;
	}
	.dm-back-btn:hover:not(:disabled) {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}
	.dm-back-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.dm-center-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		min-height: 240px;
		gap: 14px;
		color: var(--text-secondary);
		font-size: 0.85rem;
		text-transform: uppercase;
		letter-spacing: 1px;
	}
</style>
