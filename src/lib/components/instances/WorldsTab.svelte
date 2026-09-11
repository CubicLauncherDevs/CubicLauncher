<script lang="ts">
	import { onDestroy, untrack } from "svelte";
	import { convertFileSrc } from "@tauri-apps/api/core";
	import { open, save } from "@tauri-apps/plugin-dialog";
	import {
		getInstanceWorlds,
		worldAction,
		type WorldAction,
		type WorldDto,
		type WorldProgress,
	} from "$lib/api/worlds";
	import { InstState, type InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import WorldIcon from "$lib/icons/WorldIcon.svelte";
	import Select from "$lib/components/layout/Select.svelte";
	import ChevronDownIcon from "$lib/icons/ChevronDownIcon.svelte";
	import Lupa from "$lib/icons/Lupa.svelte";
	import CloseIcon from "$lib/icons/CloseIcon.svelte";

	let { instance } = $props<{ instance: InstanceDto }>();
	let worlds = $state<WorldDto[]>([]);
	let loading = $state(true);
	let operation = $state<string | null>(null);
	let error = $state("");
	let notice = $state("");
	let progress = $state<WorldProgress>({ bytes: 0, files: 0 });
	let search = $state("");
	let sort = $state("lastPlayed");
	let filtersOpen = $state(true);
	let page = $state(0);
	let selectedFolder = $state<string | null>(null);
	let sizes = $state<Record<string, number>>({});
	let modalOpen = $state(false);
	let modalAction = $state<"rename" | "delete">("rename");
	let modalWorld = $state<WorldDto | null>(null);
	let newName = $state("");
	let alive = true;
	let request = 0;
	const PAGE_SIZE = 50;
	const running = $derived(
		instance.status === InstState.Started ||
			instance.status === InstState.Starting,
	);
	const disabled = $derived(loading || operation !== null);
	const filtered = $derived.by(() => {
		const query = search.trim().toLocaleLowerCase();
		return worlds
			.filter((world) =>
				`${world.name} ${world.folder}`
					.toLocaleLowerCase()
					.includes(query),
			)
			.sort((a, b) =>
				sort === "name"
					? a.name.localeCompare(b.name)
					: (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0) ||
						a.folder.localeCompare(b.folder),
			);
	});
	const pages = $derived(Math.max(1, Math.ceil(filtered.length / PAGE_SIZE)));
	const visible = $derived(
		filtered.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE),
	);
	const selected = $derived(
		worlds.find((world) => world.folder === selectedFolder),
	);

	onDestroy(() => {
		alive = false;
		request++;
	});

	async function load() {
		const token = ++request;
		loading = true;
		error = "";
		try {
			const result = await getInstanceWorlds(instance.uuid);
			if (!alive || token !== request) return;
			worlds = result;
			if (!result.some((world) => world.folder === selectedFolder))
				selectedFolder = result[0]?.folder ?? null;
			page = Math.min(
				page,
				Math.max(0, Math.ceil(result.length / PAGE_SIZE) - 1),
			);
		} catch (err) {
			if (alive && token === request) error = String(err);
		} finally {
			if (alive && token === request) loading = false;
		}
	}

	$effect(() => {
		// Refresh on mount and when Minecraft exits; no polling or background watcher.
		const isRunning = running;
		untrack(() => {
			if (!isRunning || worlds.length === 0) void load();
		});
	});

	$effect(() => {
		if (page >= pages) page = pages - 1;
	});

	async function perform(
		label: string,
		getAction: () => Promise<WorldAction | null> | WorldAction,
	) {
		if (disabled) return;
		operation = label;
		error = "";
		notice = "";
		progress = { bytes: 0, files: 0 };
		try {
			const action = await getAction();
			if (!action || !alive) return;
			const result = await worldAction(instance.uuid, action, (event) => {
				if (alive) progress = event;
			});
			if (!alive) return;
			if (action.type === "size" && result.size !== null)
				sizes[action.folder] = result.size;
			if (
				["import", "duplicate", "rename", "delete", "resetIcon"].includes(
					action.type,
				)
			) {
				if (result.folder) selectedFolder = result.folder;
				sizes = {};
				modalOpen = false;
				await load();
			}
			if (!["open", "openDatapacks", "copySeed"].includes(action.type))
				notice = t("worlds.completed");
		} catch (err) {
			if (alive) error = String(err);
		} finally {
			if (alive) operation = null;
		}
	}

	function importWorld(directory: boolean) {
		void perform(t("worlds.importing"), async () => {
			const path = await open({
				directory,
				multiple: false,
				title: t(
					directory ? "worlds.importFolder" : "worlds.importZip",
				),
				...(!directory
					? { filters: [{ name: "ZIP", extensions: ["zip"] }] }
					: {}),
			});
			return typeof path === "string" ? { type: "import", path } : null;
		});
	}

	function exportWorld(world: WorldDto) {
		void perform(t("worlds.exporting"), async () => {
			const path = await save({
				defaultPath: `${world.folder}.zip`,
				filters: [{ name: "ZIP", extensions: ["zip"] }],
				title: t("worlds.export"),
			});
			return path ? { type: "export", folder: world.folder, path } : null;
		});
	}

	function showModal(action: "rename" | "delete", world: WorldDto) {
		modalAction = action;
		modalWorld = world;
		newName = world.name;
		modalOpen = true;
	}

	function submitModal() {
		const world = modalWorld;
		if (!world || running) return;
		void perform(t(`worlds.${modalAction}`), () =>
			modalAction === "rename"
				? { type: "rename", folder: world.folder, name: newName }
				: { type: "delete", folder: world.folder },
		);
	}

	function formatBytes(bytes: number) {
		const unit = Math.min(
			4,
			Math.floor(Math.log2(Math.max(1, bytes)) / 10),
		);
		return `${(bytes / 1024 ** unit).toLocaleString(undefined, { maximumFractionDigits: 1 })} ${["B", "KiB", "MiB", "GiB", "TiB"][unit]}`;
	}

	async function copyToClipboard(text: string) {
		try {
			await navigator.clipboard.writeText(text);
			return;
		} catch {
			const textarea = document.createElement("textarea");
			textarea.value = text;
			textarea.style.position = "fixed";
			textarea.style.opacity = "0";
			document.body.appendChild(textarea);
			textarea.select();
			const success = document.execCommand("copy");
			document.body.removeChild(textarea);
			if (!success) throw new Error(t("worlds.copySeedFailed"));
		}
	}

	async function copySeed(world: WorldDto) {
		if (disabled || !world.seed) return;
		operation = t("worlds.copyingSeed");
		error = "";
		notice = "";
		try {
			const result = await worldAction(
				instance.uuid,
				{ type: "copySeed", folder: world.folder },
				() => {},
			);
			if (!alive) return;
			const seed = result.seed ?? world.seed;
			if (seed) {
				await copyToClipboard(seed);
				notice = t("worlds.seedCopied");
			} else {
				notice = t("worlds.noSeed");
			}
		} catch (err) {
			if (alive) error = String(err);
		} finally {
			if (alive) operation = null;
		}
	}

	function lastPlayed(world: WorldDto) {
		if (!world.lastPlayed) return t("worlds.unknown");
		const date = new Date(world.lastPlayed);
		return Number.isNaN(date.getTime())
			? t("worlds.unknown")
			: date.toLocaleString();
	}

	function gameMode(world: WorldDto) {
		if (world.hardcore) return t("worlds.hardcore");
		return t(
			`worlds.${["survival", "creative", "adventure", "spectator"][world.gameMode ?? -1] ?? "unknown"}`,
		);
	}
</script>

<section class="worlds" aria-label={t("worlds.title")}>
	<div class="toolbar">
		<h2>
			<WorldIcon size={22} />
			{t("worlds.title")} <span>{worlds.length}</span>
		</h2>
		<div class="actions">
			<button
				type="button"
				disabled={disabled || running}
				onclick={() => importWorld(false)}
				>{t("worlds.importZip")}</button
			>
			<button
				type="button"
				disabled={disabled || running}
				onclick={() => importWorld(true)}
				>{t("worlds.importFolder")}</button
			>
			<button
				type="button"
				{disabled}
				onclick={() =>
					perform(t("worlds.openFolder"), () => ({
						type: "open",
						folder: null,
					}))}>{t("worlds.openSaves")}</button
			>
			<button
				type="button"
				{disabled}
				onclick={() => {
					sizes = {};
					void load();
				}}>{t("worlds.refresh")}</button
			>
		</div>
	</div>
	{#if running}<p class="hint">{t("worlds.running")}</p>{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
	{#if operation}
		<div class="progress" role="status">
			<progress aria-label={operation}></progress><span
				>{operation} · {formatBytes(progress.bytes)} · {t(
					"worlds.files",
					{ count: progress.files },
				)}</span
			>
		</div>
	{:else if notice}<p class="hint" role="status">{notice}</p>{/if}
	<div class="filter-panel">
		<button
			type="button"
			class="filter-toggle"
			class:active={search.trim() !== "" || sort !== "lastPlayed"}
			aria-expanded={filtersOpen}
			aria-controls="world-filters"
			title={t(
				filtersOpen
					? "market.filter.hideFilters"
					: "market.filter.showFilters",
			)}
			onclick={() => (filtersOpen = !filtersOpen)}
		>
			<span>{t("market.filter.title")}</span>
			<span class="filter-chevron" class:expanded={filtersOpen}
				><ChevronDownIcon size={16} /></span
			>
		</button>
		<div id="world-filters" class="filters" hidden={!filtersOpen}>
			<div class="search-field">
				<Lupa width={16} height={16} />
				<input
					type="search"
					bind:value={search}
					oninput={() => (page = 0)}
					placeholder={t("worlds.search")}
					aria-label={t("worlds.search")}
				/>
				<button
					type="button"
					class="search-clear"
					disabled={!search}
					aria-label={t("market.filter.clearSearch")}
					onclick={() => {
						search = "";
						page = 0;
					}}
				>
					<CloseIcon size={14} />
				</button>
			</div>
			<div class="sort-field" role="group" aria-label={t("worlds.sort")}>
				<Select
					bind:value={sort}
					options={[
						{ value: "lastPlayed", label: t("worlds.lastPlayed") },
						{ value: "name", label: t("worlds.name") },
					]}
					onchange={() => (page = 0)}
				/>
			</div>
		</div>
	</div>
	{#if loading && worlds.length === 0}
		<p class="empty" role="status">{t("worlds.loading")}</p>
	{:else if filtered.length === 0}
		<p class="empty">
			{t(worlds.length ? "worlds.noResults" : "worlds.empty")}
		</p>
	{:else}
		<div class="browser" aria-busy={loading}>
			<div class="world-list">
				{#each visible as world (world.folder)}
					<button
						type="button"
						class="world-row"
						class:selected={selectedFolder === world.folder}
						aria-pressed={selectedFolder === world.folder}
						onclick={() => (selectedFolder = world.folder)}
					>
						<div class="world-icon">
							{#if world.icon}{#key world.iconRevision}<img
										src={`${convertFileSrc(world.icon)}?v=${encodeURIComponent(world.iconRevision ?? "")}`}
										alt=""
										loading="lazy"
										onerror={(event) => {
											(
												event.currentTarget as HTMLImageElement
											).style.display = "none";
										}}
									/>{/key}{/if}
							<WorldIcon size={28} />
						</div>
						<div class="world-label">
							<strong>{world.name}</strong><span
								>{world.folder}</span
							><small
								>{world.version ?? t("worlds.unknown")} · {gameMode(
									world,
								)}</small
							>
						</div>
						{#if world.metadataError}<span
								class="badge"
								title={t("worlds.metadataError")}>!</span
							>{/if}
					</button>
				{/each}
				{#if pages > 1}
					<div class="pagination">
						<button
							type="button"
							disabled={page === 0}
							onclick={() => page--}
							>{t("worlds.previous")}</button
						><span>{page + 1} / {pages}</span><button
							type="button"
							disabled={page + 1 >= pages}
							onclick={() => page++}>{t("worlds.next")}</button
						>
					</div>
				{/if}
			</div>
			{#if selected}
				<aside class="details">
					<h3>{selected.name}</h3>
					<dl>
						<dt>{t("worlds.folder")}</dt>
						<dd>{selected.folder}</dd>
						<dt>{t("worlds.version")}</dt>
						<dd>{selected.version ?? t("worlds.unknown")}</dd>
						<dt>{t("worlds.gameMode")}</dt>
						<dd>{gameMode(selected)}</dd>
						<dt>{t("worlds.lastPlayed")}</dt>
						<dd>{lastPlayed(selected)}</dd>
						<dt>{t("worlds.seed")}</dt>
						<dd>{selected.seed ?? t("worlds.unknown")}</dd>
						<dt>{t("worlds.size")}</dt>
						<dd>
							{sizes[selected.folder] === undefined
								? t("worlds.notCalculated")
								: formatBytes(sizes[selected.folder])}
						</dd>
					</dl>
					{#if selected.metadataError}<p class="hint">
							{t("worlds.metadataError")}
						</p>{/if}
					<div class="detail-actions">
						<button
							type="button"
							{disabled}
							onclick={() =>
								selected &&
								perform(t("worlds.openFolder"), () => ({
									type: "open",
									folder: selected!.folder,
								}))}>{t("worlds.openFolder")}</button
						>
						<button
							type="button"
							disabled={disabled || running}
							onclick={() =>
								selected &&
								perform(t("worlds.calculating"), () => ({
									type: "size",
									folder: selected!.folder,
								}))}>{t("worlds.calculateSize")}</button
						>
						<button
							type="button"
							disabled={disabled || running}
							onclick={() => selected && exportWorld(selected)}
							>{t("worlds.export")}</button
						>
						<button
							type="button"
							disabled={disabled || running}
							onclick={() =>
								selected &&
								perform(t("worlds.duplicating"), () => ({
									type: "duplicate",
									folder: selected!.folder,
								}))}>{t("worlds.duplicate")}</button
						>
						<button
							type="button"
							disabled={disabled ||
								running ||
								selected.metadataError}
							onclick={() =>
								selected && showModal("rename", selected)}
							>{t("worlds.rename")}</button
						>
						<button
							type="button"
							disabled={disabled || running}
							onclick={() =>
								selected &&
								perform(t("worlds.openingDatapacks"), () => ({
									type: "openDatapacks",
									folder: selected!.folder,
								}))}
							>{t("worlds.openDatapacks")}</button
						>
						<button
							type="button"
							disabled={disabled || running || !selected.seed}
							onclick={() => selected && copySeed(selected)}
							>{t("worlds.copySeed")}</button
						>
						<button
							type="button"
							disabled={disabled || running}
							onclick={() =>
								selected &&
								perform(t("worlds.resettingIcon"), () => ({
									type: "resetIcon",
									folder: selected!.folder,
								}))}
							>{t("worlds.resetIcon")}</button
						>
						<button
							type="button"
							class="danger"
							disabled={disabled || running}
							onclick={() =>
								selected && showModal("delete", selected)}
							>{t("worlds.delete")}</button
						>
					</div>
				</aside>
			{/if}
		</div>
	{/if}
</section>

<ModalBase
	bind:open={modalOpen}
	title={t(`worlds.${modalAction}`)}
	width="460px"
>
	<form
		onsubmit={(event) => {
			event.preventDefault();
			submitModal();
		}}
	>
		{#if modalAction === "rename"}
			<label for="world-name">{t("worlds.name")}</label>
			<input
				id="world-name"
				bind:value={newName}
				required
				maxlength={256}
				disabled={operation !== null}
			/>
			<p class="hint">{t("worlds.renameHint")}</p>
		{:else}
			<p>
				{t("worlds.deleteConfirm", {
					name: modalWorld?.name ?? "",
					folder: modalWorld?.folder ?? "",
				})}
			</p>
		{/if}
		{#if error}<p class="error" role="alert">{error}</p>{/if}
		<div class="modal-actions">
			<button type="button" onclick={() => (modalOpen = false)}
				>{t("common.cancel")}</button
			><button
				class:danger={modalAction === "delete"}
				type="submit"
				disabled={disabled ||
					running ||
					(modalAction === "rename" && !newName.trim())}
				>{operation ?? t(`worlds.${modalAction}`)}</button
			>
		</div>
	</form>
</ModalBase>

<style>
	.worlds {
		display: flex;
		flex-direction: column;
		gap: 16px;
		color: var(--text-primary);
	}
	.toolbar,
	.actions,
	h2,
	.filters,
	.pagination,
	.modal-actions {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.toolbar {
		justify-content: space-between;
		flex-wrap: wrap;
	}
	h2 {
		margin: 0;
		font-size: 1.15rem;
	}
	h2 span {
		color: var(--text-tertiary);
		font-size: 0.85rem;
	}
	.actions {
		flex-wrap: wrap;
	}
	button,
	input {
		font: inherit;
		font-size: 0.82rem;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
		background: var(--bg-card);
		padding: 9px 12px;
	}
	button {
		cursor: pointer;
	}
	button:hover:not(:disabled) {
		background: var(--bg-item-active);
		border-color: var(--accent-primary);
	}
	button:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}
	button:focus-visible,
	input:focus-visible {
		outline: 2px solid var(--accent-primary);
		outline-offset: 2px;
	}
	.filter-panel {
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card);
	}
	.filter-toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		width: 100%;
		padding: 12px 14px;
		border: none;
		border-radius: var(--border-radius);
		background: transparent;
		color: var(--text-secondary);
		font-weight: 600;
		text-align: left;
		transition:
			background 0.15s ease,
			color 0.15s ease;
	}
	.filter-toggle.active {
		color: var(--accent-primary);
		background: color-mix(in srgb, var(--accent-primary) 8%, transparent);
	}
	.filter-chevron {
		display: flex;
		transition: transform 0.15s ease;
	}
	.filter-chevron.expanded {
		transform: rotate(180deg);
	}
	.filters {
		flex-wrap: wrap;
		padding: 12px 14px;
		border-top: 1px solid var(--border);
	}
	.filters[hidden] {
		display: none;
	}
	.search-field {
		display: flex;
		align-items: center;
		gap: 10px;
		flex: 1 1 220px;
		min-width: 0;
		min-height: 40px;
		padding: 0 10px 0 12px;
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-tertiary);
		transition:
			border-color 0.15s ease,
			background 0.15s ease;
	}
	.search-field:focus-within {
		border-color: var(--accent-primary);
		box-shadow: 0 0 0 2px
			color-mix(in srgb, var(--accent-primary) 12%, transparent);
	}
	.search-field input {
		flex: 1;
		min-width: 0;
		padding: 10px 0;
		border: none;
		background: transparent;
		outline: none;
	}
	.search-field input::placeholder {
		color: var(--text-tertiary);
	}
	.search-field input::-webkit-search-cancel-button {
		-webkit-appearance: none;
	}
	.search-clear {
		display: grid;
		place-items: center;
		flex-shrink: 0;
		padding: 6px;
		border: none;
		background: transparent;
		color: var(--text-secondary);
	}
	.sort-field {
		flex: 0 1 200px;
		min-width: 160px;
	}
	.sort-field :global(.select-trigger) {
		min-height: 40px;
		font-size: 0.82rem;
	}
	.sort-field :global(.select-trigger:focus-visible) {
		outline: 2px solid var(--accent-primary);
		outline-offset: 2px;
	}
	@media (prefers-reduced-motion: reduce) {
		.filter-toggle,
		.filter-chevron,
		.search-field {
			transition: none;
		}
	}
	.browser {
		display: grid;
		grid-template-columns: minmax(240px, 1fr) minmax(240px, 320px);
		gap: 20px;
		align-items: start;
	}
	.world-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-width: 0;
	}
	.world-row {
		display: flex;
		align-items: center;
		gap: 12px;
		text-align: left;
		padding: 12px;
		min-height: 84px;
	}
	.world-row.selected {
		border-color: var(--accent-primary);
		background: var(--bg-item-active);
	}
	.world-icon {
		position: relative;
		display: grid;
		place-items: center;
		width: 54px;
		height: 54px;
		flex-shrink: 0;
		background: var(--bg-sidebar);
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		color: var(--text-tertiary);
	}
	.world-icon img {
		position: absolute;
		width: 100%;
		height: 100%;
		object-fit: cover;
		z-index: 1;
	}
	.world-label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
		flex: 1;
	}
	.world-label strong,
	.world-label span,
	.world-label small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.world-label span,
	.world-label small,
	dt,
	.hint {
		color: var(--text-secondary);
		font-size: 0.8rem;
	}
	.badge {
		color: var(--text-secondary);
		border: 1px solid var(--border);
		padding: 2px 7px;
		border-radius: 50%;
	}
	.details {
		position: sticky;
		top: 0;
		padding: 18px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card);
		overflow-wrap: anywhere;
	}
	h3 {
		margin: 0 0 16px;
		font-size: 1rem;
	}
	dl {
		margin: 0;
	}
	dt {
		margin-top: 12px;
	}
	dd {
		margin: 4px 0 0;
		font-size: 0.85rem;
	}
	.detail-actions {
		margin-top: 20px;
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}
	.danger {
		color: var(--error, #ef6464);
	}
	.error {
		color: var(--error, #ef6464);
		overflow-wrap: anywhere;
	}
	.hint,
	.error {
		margin: 0;
		line-height: 1.5;
	}
	.details .hint {
		margin-top: 14px;
	}
	.empty {
		padding: 56px 20px;
		text-align: center;
		color: var(--text-secondary);
	}
	.progress {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
		font-size: 0.82rem;
	}
	progress {
		width: 120px;
		height: 6px;
		accent-color: var(--accent-primary);
	}
	.pagination {
		justify-content: center;
		margin-top: 8px;
		font-size: 0.8rem;
	}
	form {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}
	form p {
		line-height: 1.5;
	}
	label {
		font-size: 0.85rem;
	}
	.modal-actions {
		justify-content: flex-end;
		margin-top: 10px;
	}
	@media (max-width: 700px) {
		.browser {
			grid-template-columns: 1fr;
		}
		.details {
			position: static;
			grid-row: 1;
		}
	}
</style>
