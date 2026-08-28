<script lang="ts">
	import { t } from "$lib/i18n";
	import Loading from "$lib/icons/Loading.svelte";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import type { ModDownloadInfo } from "$lib/api/cubicApi";
	import type {
		DependencyConflict,
		ResolvedDependency,
	} from "$lib/types/dependency";
	import { SvelteSet } from "svelte/reactivity";

	let {
		open = $bindable(false),
		projectTitle,
		tree = [],
		conflicts = [],
		installedProjectIds = new Set<string>(),
		resolving = false,
		downloading = false,
		error = null,
		onConfirm,
		onCancel,
		onclose,
	}: {
		open?: boolean;
		projectTitle: string;
		tree?: ResolvedDependency[];
		conflicts?: DependencyConflict[];
		installedProjectIds?: Set<string>;
		resolving?: boolean;
		downloading?: boolean;
		error?: string | null;
		onConfirm?: (queue: ModDownloadInfo[]) => void;
		onCancel?: () => void;
		onclose?: () => void;
	} = $props();

	let selectedOptionalIds = new SvelteSet<string>();
	let queue = $state<ModDownloadInfo[]>([]);

	$effect(() => {
		// Clean optional selections when the dependency tree changes
		void tree;
		selectedOptionalIds.clear();
	});

	$effect(() => {
		// Recompute download queue whenever the tree or optional selections
		// change so the displayed count and the confirmed queue stay in sync.
		void selectedOptionalIds.size;
		void tree;
		queue = computeQueue(tree);
	});

	function isInstalled(dep: ResolvedDependency): boolean {
		return installedProjectIds.has(dep.project_id);
	}

	function badgeLabel(kind: ResolvedDependency["kind"]): string {
		const key = `market.detail.dependencies.dependencyKind.${kind}`;
		const value = t(key as never);
		return value === key ? kind : value;
	}

	function toggleOptional(projectId: string) {
		if (selectedOptionalIds.has(projectId)) {
			selectedOptionalIds.delete(projectId);
		} else {
			selectedOptionalIds.add(projectId);
		}
	}

	function hasVisibleChildren(dep: ResolvedDependency): boolean {
		if (
			dep.kind === "optional" &&
			!selectedOptionalIds.has(dep.project_id)
		) {
			return false;
		}
		return dep.children.length > 0;
	}

	function computeQueue(deps: ResolvedDependency[]): ModDownloadInfo[] {
		const result: ModDownloadInfo[] = [];
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
				if (!dep.download_url || !dep.filename) {
					continue;
				}
				const key = dep.filename.toLowerCase();
				if (seen.has(key)) continue;

				seen.add(key);
				result.push({
					url: dep.download_url,
					filename: dep.filename,
					projectTitle: dep.title,
					iconUrl: dep.icon_url ?? undefined,
					project_id: dep.project_id,
					version_id: dep.version_id ?? undefined,
				});
				walk(dep.children);
			}
		}

		walk(deps);
		return result;
	}

	function handleConfirm() {
		if (queue.length === 0) return;
		onConfirm?.(queue);
	}

	function handleCancel() {
		open = false;
		onCancel?.();
	}

	function installButtonLabel(count: number): string {
		if (count === 1) {
			return t("market.detail.dependencies.installSingle");
		}
		return t("market.detail.dependencies.install", { count });
	}

	const queueCount = $derived(queue.length);
</script>

<ModalBase
	bind:open
	title={t("market.detail.dependencies.title", { title: projectTitle })}
	width="650px"
	onclose={() => {
		onclose?.();
	}}
>
	<div class="market-dep-preview">
		{#if resolving}
			<div class="market-dep-state">
				<Loading />
				<p>{t("market.detail.dependencies.resolving")}</p>
			</div>
		{:else if error}
			<div class="market-dep-state market-dep-error">
				{error}
			</div>
		{:else if tree.length === 0}
			<div class="market-dep-state">
				{t("market.detail.dependencies.noDependencies", {
					title: projectTitle,
				})}
			</div>
		{:else}
			{#if conflicts.length > 0}
				<div class="market-conflicts-box">
					<strong>
						{t("market.detail.dependencies.conflictsTitle")}
					</strong>
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

			<div class="market-dep-list">
				{#each tree as dep (dep.project_id)}
					{@render dependencyNode(dep)}
				{/each}
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<div class="market-dep-footer">
			<button
				type="button"
				class="market-detail-btn secondary"
				onclick={handleCancel}
				disabled={resolving}
			>
				{t("market.detail.dependencies.cancel")}
			</button>
			<button
				type="button"
				class="market-detail-btn primary"
				disabled={resolving || downloading || queueCount === 0}
				onclick={handleConfirm}
			>
				{installButtonLabel(queueCount)}
			</button>
		</div>
	{/snippet}
</ModalBase>

{#snippet dependencyNode(dep: ResolvedDependency)}
	{@const isOptional = dep.kind === "optional"}
	{@const isIncompatible = dep.kind === "incompatible"}
	{@const installed = isInstalled(dep)}
	{@const visibleChildren = hasVisibleChildren(dep)}

	<div
		class="market-dep-node"
		class:optional={isOptional}
		class:incompatible={isIncompatible}
	>
		<div class="market-dep-row" style:padding-left="{dep.depth * 14}px">
			<div class="market-dep-icon">
				{#if dep.icon_url}
					<img src={dep.icon_url} alt="" />
				{:else}
					<span>🧩</span>
				{/if}
			</div>
			<div class="market-dep-info">
				<span class="market-dep-title">{dep.title}</span>
				{#if dep.filename}
					<span class="market-dep-filename">{dep.filename}</span>
				{/if}
			</div>
			<span class="market-dep-badge market-dep-badge-{dep.kind}">
				{badgeLabel(dep.kind)}
			</span>
			{#if installed}
				<span class="market-dep-badge market-dep-badge-installed">
					{t("market.detail.dependencies.installed")}
				</span>
			{:else if isOptional}
				<label class="market-optional-toggle">
					<input
						type="checkbox"
						checked={selectedOptionalIds.has(dep.project_id)}
						onchange={() => toggleOptional(dep.project_id)}
					/>
					{t("market.detail.dependencies.include")}
				</label>
			{/if}
		</div>
		{#if visibleChildren}
			<div class="market-dep-children">
				{#each dep.children as child (child.project_id)}
					{@render dependencyNode(child)}
				{/each}
			</div>
		{/if}
	</div>
{/snippet}

<style>
	.market-dep-preview {
		min-height: 220px;
		max-height: 50vh;
		overflow-y: auto;
		padding: 4px;
	}

	.market-dep-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 14px;
		color: var(--text-secondary);
		font-size: 0.85rem;
		text-align: center;
		min-height: 220px;
	}

	.market-dep-error {
		color: var(--color-error);
	}

	.market-conflicts-box {
		background: rgba(var(--color-error-rgb), 0.08);
		border: 1px solid rgba(var(--color-error-rgb), 0.25);
		border-radius: var(--border-radius-sm);
		padding: 12px 14px;
		margin-bottom: 16px;
		color: var(--color-error);
		font-size: 0.78rem;
	}
	.market-conflicts-box strong {
		display: block;
		margin-bottom: 6px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		font-size: 0.65rem;
	}
	.market-conflicts-box ul {
		margin: 0;
		padding-left: 16px;
	}
	.market-conflicts-box li {
		margin-bottom: 2px;
	}

	.market-dep-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.market-dep-node {
		display: flex;
		flex-direction: column;
	}
	.market-dep-node.incompatible {
		opacity: 0.6;
	}
	.market-dep-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		min-height: 42px;
	}
	.market-dep-row:hover {
		background: rgba(255, 255, 255, 0.04);
	}

	.market-dep-icon {
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
	.market-dep-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.market-dep-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
		gap: 1px;
	}
	.market-dep-title {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.market-dep-filename {
		font-size: 0.72rem;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.market-dep-badge {
		font-size: 0.58rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.4px;
		padding: 2px 6px;
		border-radius: 4px;
		flex-shrink: 0;
	}
	.market-dep-badge-required {
		background: rgba(var(--color-success-rgb), 0.12);
		color: var(--color-success);
	}
	.market-dep-badge-embedded {
		background: rgba(var(--color-info-rgb, 59 130 246), 0.12);
		color: var(--color-info, #3b82f6);
	}
	.market-dep-badge-optional {
		background: rgba(var(--color-warning-rgb), 0.12);
		color: var(--color-warning);
	}
	.market-dep-badge-incompatible {
		background: rgba(var(--color-error-rgb), 0.12);
		color: var(--color-error);
	}
	.market-dep-badge-installed {
		background: rgba(var(--color-success-rgb), 0.12);
		color: var(--color-success);
	}
	.market-optional-toggle {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 0.7rem;
		color: var(--text-secondary);
		cursor: pointer;
		flex-shrink: 0;
	}
	.market-optional-toggle input {
		cursor: pointer;
	}
	.market-dep-children {
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin-top: 2px;
	}

	.market-dep-footer {
		display: flex;
		justify-content: flex-end;
		gap: 10px;
		padding-top: 14px;
		border-top: 1px solid var(--border);
		margin-top: 12px;
	}

	.market-detail-btn {
		padding: 8px 16px;
		border-radius: var(--border-radius-sm);
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
		border: 1px solid transparent;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			border-color 0.15s ease,
			opacity 0.15s ease;
	}

	.market-detail-btn.secondary {
		background: var(--button-bg);
		color: var(--text-primary);
		border-color: var(--border);
	}

	.market-detail-btn.secondary:hover:not(:disabled) {
		background: var(--button-hover-bg);
	}

	.market-detail-btn.primary {
		background: var(--accent);
		color: var(--accent-text, #ffffff);
	}

	.market-detail-btn.primary:hover:not(:disabled) {
		filter: brightness(1.1);
	}

	.market-detail-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
