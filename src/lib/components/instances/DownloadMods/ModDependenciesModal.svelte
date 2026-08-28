<script lang="ts">
	import { t } from "$lib/i18n";
	import Loading from "$lib/icons/Loading.svelte";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import { resolveModDependencies } from "$lib/api/cubicApi";
	import type {
		DependencyConflict,
		DependencyRequest,
		ResolvedDependency,
	} from "$lib/types/dependency";
	import { SvelteSet } from "svelte/reactivity";

	let {
		open = $bindable(false),
		request,
		loader,
		gameVersion,
		projectTitle,
		installedProjectIds = new Set<string>(),
	}: {
		open: boolean;
		request: DependencyRequest | null;
		loader: string;
		gameVersion: string;
		projectTitle: string;
		installedProjectIds?: Set<string>;
	} = $props();

	let resolving = $state(false);
	let tree = $state<ResolvedDependency[]>([]);
	let conflicts = $state<DependencyConflict[]>([]);
	let error = $state<string | null>(null);
	let selectedOptionalIds = new SvelteSet<string>();

	$effect(() => {
		if (open && request) {
			loadDependencies();
		}
	});

	async function loadDependencies() {
		if (!request) return;

		resolving = true;
		error = null;
		tree = [];
		conflicts = [];
		selectedOptionalIds.clear();

		try {
			const result = await resolveModDependencies(
				[request],
				loader,
				gameVersion,
			);
			tree = result.tree;
			conflicts = result.conflicts;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			resolving = false;
		}
	}

	function badgeLabel(kind: ResolvedDependency["kind"]): string {
		const key = `instanceView.downloadMods.dependencyKind.${kind}`;
		const value = t(key as never);
		return value === key ? kind : value;
	}

	function isInstalled(dep: ResolvedDependency): boolean {
		return installedProjectIds.has(dep.project_id);
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
</script>

<ModalBase
	bind:open
	title={t("instanceView.downloadMods.dependenciesTitle", {
		title: projectTitle,
	})}
	width="650px"
>
	<div class="dm-dep-preview">
		{#if resolving}
			<div class="dm-preview-state">
				<Loading />
				<p>{t("instanceView.downloadMods.resolvingDeps")}</p>
			</div>
		{:else if error}
			<div class="dm-preview-state dm-preview-error">
				{error}
			</div>
		{:else if tree.length === 0}
			<div class="dm-preview-state">
				{t("instanceView.downloadMods.noDependencies")}
			</div>
		{:else}
			{#if conflicts.length > 0}
				<div class="dm-conflicts-box">
					<strong>
						{t("instanceView.downloadMods.conflictsTitle")}
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

			<div class="dm-dep-list">
				{#each tree as dep (dep.project_id)}
					{@render dependencyNode(dep)}
				{/each}
			</div>
		{/if}
	</div>
</ModalBase>

{#snippet dependencyNode(dep: ResolvedDependency)}
	{@const isOptional = dep.kind === "optional"}
	{@const isIncompatible = dep.kind === "incompatible"}
	{@const installed = isInstalled(dep)}
	{@const visibleChildren = hasVisibleChildren(dep)}

	<div
		class="dm-dep-node"
		class:optional={isOptional}
		class:incompatible={isIncompatible}
	>
		<div class="dm-dep-row" style:padding-left="{dep.depth * 14}px">
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
		{#if visibleChildren}
			<div class="dm-dep-children">
				{#each dep.children as child (child.project_id)}
					{@render dependencyNode(child)}
				{/each}
			</div>
		{/if}
	</div>
{/snippet}

<style>
	.dm-dep-preview {
		min-height: 220px;
		max-height: 60vh;
		overflow-y: auto;
		padding: 4px;
	}

	.dm-preview-state {
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

	.dm-preview-error {
		color: var(--color-error);
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

	.dm-dep-list {
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
</style>
