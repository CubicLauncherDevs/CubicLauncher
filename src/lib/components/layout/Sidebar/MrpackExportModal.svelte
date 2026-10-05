<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { SvelteMap, SvelteSet } from "svelte/reactivity";
	import { save } from "@tauri-apps/plugin-dialog";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import { t } from "$lib/i18n";
	import { showSuccess } from "$lib/state/state.svelte";
	import {
		previewMrpackExport,
		exportInstanceMrpack,
		type MrpackExportEntry,
		type MrpackExportPreview,
	} from "$lib/api/mrpackExportApi";

	let { open = $bindable(), id }: { open: boolean; id: string } = $props();
	let preview = $state.raw<MrpackExportPreview | null>(null);
	let name = $state("");
	let versionId = $state("");
	let summary = $state("");
	let author = $state("");
	let loading = $state(true);
	let working = $state(false);
	let error = $state("");
	let selected = $state<Set<string>>(new Set());
	let disposed = false;

	interface TreeNode {
		entry: MrpackExportEntry;
		label: string;
		children: TreeNode[];
		leaves: string[];
	}

	const files = $derived(
		preview?.entries.filter((entry) => !entry.isDir) ?? [],
	);
	const selectedSize = $derived(
		files.reduce(
			(size, file) => size + (selected.has(file.path) ? file.size : 0),
			0,
		),
	);
	const tree = $derived.by(() => {
		const roots: TreeNode[] = [];
		const nodes = new SvelteMap<string, TreeNode>();
		for (const entry of preview?.entries ?? []) {
			const node: TreeNode = {
				entry,
				label: entry.path.split("/").at(-1) ?? entry.path,
				children: [],
				leaves: entry.isDir ? [] : [entry.path],
			};
			nodes.set(entry.path, node);
			const parent = nodes.get(
				entry.path.slice(0, entry.path.lastIndexOf("/")),
			);
			if (entry.path.includes("/") && parent) parent.children.push(node);
			else roots.push(node);
		}
		for (const node of [...nodes.values()].reverse()) {
			if (node.entry.isDir)
				node.leaves = node.children.flatMap((child) => child.leaves);
		}
		return roots;
	});

	onMount(() => void load());
	onDestroy(() => {
		disposed = true;
	});

	async function load() {
		loading = true;
		error = "";
		try {
			const result = await previewMrpackExport(id);
			if (disposed) return;
			preview = result;
			name = result.name;
			versionId = result.versionId;
			summary = result.summary;
			author = result.author ?? "";
			defaults();
		} catch (e) {
			if (!disposed) error = String(e);
		} finally {
			if (!disposed) loading = false;
		}
	}

	function defaults() {
		selected = new SvelteSet(
			(preview?.entries ?? [])
				.filter((entry) => !entry.isDir && entry.defaultSelected)
				.map((entry) => entry.path),
		);
	}

	function toggle(paths: string[], checked: boolean) {
		const next = new SvelteSet(selected);
		for (const path of paths) {
			if (checked) next.add(path);
			else next.delete(path);
		}
		selected = next;
	}

	function formatSize(bytes: number) {
		const units = ["B", "KiB", "MiB", "GiB", "TiB"];
		const index =
			bytes > 0
				? Math.min(
						Math.floor(Math.log(bytes) / Math.log(1024)),
						units.length - 1,
					)
				: 0;
		return `${(bytes / 1024 ** index).toLocaleString(undefined, { maximumFractionDigits: 1 })} ${units[index]}`;
	}

	async function exportPack() {
		if (working || loading || !preview || !name.trim() || !versionId.trim())
			return;
		working = true;
		error = "";
		try {
			const filename =
				Array.from(name.trim(), (char) =>
					char.charCodeAt(0) < 32 ? "_" : char,
				)
					.join("")
					.replace(/[<>:"/\\|?*]/g, "_")
					.replace(/[. ]+$/, "") || "modpack";
			const dest = await save({
				defaultPath: `${filename}.mrpack`,
				filters: [{ name: "Modrinth Modpack", extensions: ["mrpack"] }],
			});
			if (!dest || disposed) return;
			const path = await exportInstanceMrpack(id, dest, {
				name: name.trim(),
				versionId: versionId.trim(),
				summary: summary.trim(),
				author: author.trim(),
				selectedPaths: [...selected],
			});
			showSuccess(
				t("notifications.exportTitle"),
				t("notifications.exportSuccess", { path }),
			);
			if (!disposed) open = false;
		} catch (e) {
			if (!disposed) error = String(e);
		} finally {
			if (!disposed) working = false;
		}
	}
</script>

{#snippet checkbox(node: TreeNode)}
	{@const count = node.leaves.filter((path) => selected.has(path)).length}
	<label class="file-label">
		<input
			type="checkbox"
			checked={node.leaves.length > 0 && count === node.leaves.length}
			indeterminate={count > 0 && count < node.leaves.length}
			disabled={working || node.leaves.length === 0}
			onchange={(event) =>
				toggle(node.leaves, event.currentTarget.checked)}
		/>
		<span class="filename" title={node.entry.path}>{node.label}</span>
		<span class="size">{formatSize(node.entry.size)}</span>
	</label>
{/snippet}

{#snippet branch(nodes: TreeNode[], depth: number)}
	<ul>
		{#each nodes as node (node.entry.path)}
			<li>
				{#if node.entry.isDir}
					<details open={depth === 0}>
						<summary>{@render checkbox(node)}</summary>
						{@render branch(node.children, depth + 1)}
					</details>
				{:else}
					<div class="file-row">{@render checkbox(node)}</div>
				{/if}
			</li>
		{/each}
	</ul>
{/snippet}

<ModalBase
	bind:open
	title={t("mrpackExport.title")}
	width="720px"
	closeDisabled={working}
>
	{#if loading}
		<p role="status">{t("mrpackExport.loading")}</p>
	{:else if preview}
		<fieldset disabled={working} class="metadata">
			<label
				>{t("mrpackExport.name")}<input
					bind:value={name}
					maxlength="512"
					required
				/></label
			>
			<label
				>{t("mrpackExport.version")}<input
					bind:value={versionId}
					maxlength="512"
					required
				/></label
			>
			<label>
				{t("mrpackExport.authorOptional")}
				<input bind:value={author} maxlength="512" autocomplete="off" />
			</label>
			<label
				>{t("mrpackExport.summary")}<textarea
					bind:value={summary}
					maxlength="16384"
					rows="2"></textarea></label
			>
		</fieldset>
		<p class="dependencies">
			{Object.entries(preview.dependencies)
				.map(([key, value]) => `${key}: ${value}`)
				.join(" · ")}
		</p>
		<p>{t("mrpackExport.hint")}</p>
		<div class="toolbar">
			<button
				type="button"
				class="btn-secondary"
				disabled={working}
				onclick={defaults}>{t("mrpackExport.defaults")}</button
			>
			<button
				type="button"
				class="btn-secondary"
				disabled={working}
				onclick={() =>
					toggle(
						files.map((file) => file.path),
						true,
					)}>{t("mrpackExport.selectAll")}</button
			>
			<button
				type="button"
				class="btn-secondary"
				disabled={working}
				onclick={() => (selected = new SvelteSet())}
				>{t("mrpackExport.selectNone")}</button
			>
		</div>
		<div class="file-tree" aria-label={t("mrpackExport.files")}>
			{@render branch(tree, 0)}
			{#if !files.length}<p>{t("mrpackExport.empty")}</p>{/if}
		</div>
		<p aria-live="polite">
			{t("mrpackExport.selection", {
				count: selected.size,
				size: formatSize(selectedSize),
			})}
		</p>
	{/if}
	{#if working}<p role="status">{t("mrpackExport.exporting")}</p>{/if}
	{#if error}
		<p class="error" role="alert">{error}</p>
		{#if !preview && !loading}<button
				type="button"
				class="btn-secondary"
				onclick={load}>{t("mrpackExport.retry")}</button
			>{/if}
	{/if}
	{#snippet footer()}
		<button
			type="button"
			class="btn-secondary"
			disabled={working}
			onclick={() => (open = false)}>{t("mrpackExport.cancel")}</button
		>
		<button
			type="button"
			class="btn-primary"
			disabled={loading ||
				working ||
				!preview ||
				!name.trim() ||
				!versionId.trim()}
			onclick={exportPack}>{t("mrpackExport.export")}</button
		>
	{/snippet}
</ModalBase>

<style>
	.metadata {
		display: grid;
		gap: 12px;
		border: 0;
		padding: 0;
		margin: 0;
	}
	.metadata label {
		display: grid;
		gap: 6px;
	}
	.metadata input,
	textarea {
		box-sizing: border-box;
		width: 100%;
		padding: 8px 10px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		color: var(--text-primary);
		font: inherit;
	}
	textarea {
		resize: vertical;
	}
	.dependencies,
	.size {
		color: var(--text-secondary);
		font-size: 0.85rem;
	}
	.toolbar {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-bottom: 12px;
	}
	.file-tree {
		max-height: 340px;
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 8px;
	}
	ul {
		list-style: none;
		margin: 0;
		padding-left: 16px;
	}
	.file-tree > ul {
		padding-left: 0;
	}
	summary {
		cursor: pointer;
	}
	.file-label {
		display: inline-flex;
		gap: 8px;
		align-items: center;
		width: calc(100% - 22px);
		min-height: 30px;
		vertical-align: middle;
	}
	.file-label input {
		flex-shrink: 0;
	}
	.filename {
		overflow-wrap: anywhere;
		min-width: 0;
	}
	.size {
		margin-left: auto;
		padding-left: 8px;
		white-space: nowrap;
	}
	.file-row {
		padding-left: 18px;
	}
	.file-row .file-label {
		width: 100%;
	}
	.error {
		color: var(--color-error);
		overflow-wrap: anywhere;
	}
</style>
