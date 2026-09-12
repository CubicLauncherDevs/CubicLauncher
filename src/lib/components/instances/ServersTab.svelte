<script lang="ts">
	import { onDestroy, onMount, untrack } from "svelte";
	import { t } from "$lib/i18n";
	import { InstState, type InstanceDto } from "$lib/types/types";
	import { launchInstance } from "$lib/api/cubicApi";
	import { isVersionDownloading } from "$lib/state/downloadState.svelte";
	import {
		getInstanceServers,
		serverAction,
		type ServerDto,
		type ServerStatus,
		type ServerAction,
		type ResourcePolicy,
	} from "$lib/api/servers";
	import {
		ServerResources,
		type ServerRow,
	} from "$lib/state/serverResources";
	import Icon from "$lib/icons/Icon.svelte";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import Select from "$lib/components/layout/Select.svelte";

	let { instance } = $props<{ instance: InstanceDto }>();
	let list = $state.raw<{ revision: string; servers: ServerRow[] }>({
		revision: "",
		servers: [],
	});
	let statuses = $state.raw<Record<number, ServerStatus>>({});
	let icons = $state.raw<Record<number, string | null>>({});
	let hidden = $state(false);
	let loading = $state(true);
	let working = $state(false);
	let querying = $state(false);
	let error = $state("");
	let search = $state("");
	let page = $state(0);
	let selectedIndex = $state<number | null>(null);
	let modalOpen = $state(false);
	let modalAction = $state<"add" | "edit" | "delete">("add");
	let draftName = $state("");
	let draftAddress = $state("");
	let draftPolicy = $state<ResourcePolicy>("prompt");
	let draftIndex: number | null = null;
	let draftRevision = "";
	let alive = true;
	let generation = 0;
	let loaded = false;
	// The parent keys this component by instance UUID.
	const resources = new ServerResources(
		untrack(() => instance.uuid),
		(view) => {
			if (!alive) return;
			statuses = view.statuses;
			icons = view.icons;
			querying = view.querying;
		},
		(err) => {
			if (alive) error = message(err);
		},
	);
	const PAGE_SIZE = 50;
	const running = $derived(
		instance.status === InstState.Started ||
			instance.status === InstState.Starting,
	);
	const disabled = $derived(loading || working);
	const downloading = $derived(isVersionDownloading(instance.version));
	const selected = $derived(
		list.servers.find((server) => server.index === selectedIndex),
	);
	const selectedStatus = $derived(
		selectedIndex === null ? undefined : statuses[selectedIndex],
	);
	const query = $derived(search.trim().toLocaleLowerCase());
	const filtered = $derived(
		!query
			? list.servers
			: list.servers.filter(
					(server) =>
						server.name.toLocaleLowerCase().includes(query) ||
						server.address.toLocaleLowerCase().includes(query),
				),
	);
	const pages = $derived(Math.max(1, Math.ceil(filtered.length / PAGE_SIZE)));
	const visible = $derived(
		filtered.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE),
	);
	const activeRows = $derived(
		selected && !visible.includes(selected)
			? [...visible, selected]
			: visible,
	);
	const resourceSelection = $derived(
		`${list.revision}:${activeRows.map((row) => row.key).join(",")}`,
	);

	function message(err: unknown): string {
		try {
			const parsed = JSON.parse(String(err));
			if (typeof parsed.code === "string")
				return t(`errors.${parsed.code}`, parsed.params ?? {});
		} catch {
			/* Plain transport errors are displayed directly. */
		}
		return String(err);
	}

	async function load(force = false) {
		const token = ++generation;
		if (force) resources.refresh();
		else resources.pause();
		loading = true;
		error = "";
		try {
			await resources.waitForIcons();
			if (!alive || token !== generation) return;
			const result = await getInstanceServers(instance.uuid);
			if (!alive || token !== generation) return;
			list = {
				revision: result.revision,
				servers: resources.setList(result),
			};
			loaded = true;
			if (
				!result.servers.some((server) => server.index === selectedIndex)
			)
				selectedIndex = result.servers[0]?.index ?? null;
		} catch (err) {
			if (alive && token === generation) error = message(err);
		} finally {
			if (alive && token === generation) loading = false;
		}
	}

	$effect(() => {
		const busy = running;
		untrack(() => {
			if (!busy || !loaded) void load();
		});
	});
	$effect(() => {
		if (page >= pages) page = pages - 1;
	});
	$effect(() => {
		const selection = resourceSelection;
		const paused = disabled || hidden;
		untrack(() => resources.pause());
		if (paused || selection.startsWith(":")) return;
		// Coalesce rapid typing/page changes rather than repeatedly reading icons
		// and starting connections for intermediate search results.
		const timer = setTimeout(
			() => untrack(() => resources.show(activeRows)),
			120,
		);
		return () => clearTimeout(timer);
	});
	onMount(() => {
		const update = () => {
			hidden = document.hidden;
		};
		update();
		document.addEventListener("visibilitychange", update);
		return () => document.removeEventListener("visibilitychange", update);
	});
	onDestroy(() => {
		alive = false;
		generation++;
		resources.dispose();
	});

	function showModal(action: "add" | "edit" | "delete") {
		if (disabled || running || (action !== "add" && !selected)) return;
		modalAction = action;
		draftIndex = selected?.index ?? null;
		draftRevision = list.revision;
		draftName = action === "add" ? "" : selected!.name;
		draftAddress = action === "add" ? "" : selected!.address;
		draftPolicy = action === "add" ? "prompt" : selected!.resourcePolicy;
		error = "";
		modalOpen = true;
	}

	async function perform(action: ServerAction, revision = list.revision) {
		if (disabled || running) return;
		working = true;
		error = "";
		const token = ++generation;
		resources.pause();
		try {
			await resources.waitForIcons();
			if (!alive || token !== generation) return;
			const result = await serverAction(instance.uuid, revision, action);
			if (!alive || token !== generation) return;
			list = {
				revision: result.revision,
				servers: resources.setList(result, action),
			};
			selectedIndex =
				action.type === "add"
					? result.servers.length - 1
					: action.type === "move"
						? action.index + (action.direction === "up" ? -1 : 1)
						: Math.min(action.index, result.servers.length - 1);
			if (selectedIndex < 0) selectedIndex = null;
			modalOpen = false;
		} catch (err) {
			if (alive && token === generation) error = message(err);
		} finally {
			if (alive) working = false;
		}
	}

	function submit() {
		const server = {
			name: draftName,
			address: draftAddress,
			resourcePolicy: draftPolicy,
		};
		if (modalAction === "add")
			void perform({ type: "add", server }, draftRevision);
		else if (draftIndex !== null)
			void perform(
				modalAction === "edit"
					? { type: "edit", index: draftIndex, server }
					: { type: "delete", index: draftIndex },
				draftRevision,
			);
	}

	async function connect(server: ServerDto) {
		if (disabled || running || downloading) return;
		working = true;
		error = "";
		resources.pause();
		try {
			await resources.waitForIcons();
			if (!alive) return;
			await launchInstance(
				instance,
				undefined,
				(err) => {
					if (alive) error = message(err);
				},
				server.address,
			);
		} finally {
			if (alive) working = false;
		}
	}

	function statusLabel(status?: ServerStatus) {
		return t(
			status
				? status.online
					? "servers.online"
					: "servers.unavailable"
				: querying
					? "servers.checking"
					: "servers.unknown",
		);
	}
	function playerCount(status: ServerStatus) {
		return `${status.players?.toLocaleString() ?? "?"} / ${status.maxPlayers?.toLocaleString() ?? "?"}`;
	}
</script>

<section class="servers" aria-label={t("servers.title")}>
	<div class="toolbar">
		<h2>
			<Icon name="instance:servers" size={22} />{t("servers.title")}
			<span>{list.servers.length}</span>
		</h2>
		<div class="actions">
			<button
				type="button"
				disabled={disabled || running || !list.revision}
				onclick={() => showModal("add")}
				><Icon name="nav:create" />{t("servers.add")}</button
			>
			<button type="button" {disabled} onclick={() => void load(true)}
				><Icon name="ui:refresh" />{t("servers.refresh")}</button
			>
		</div>
	</div>
	{#if running}<p class="hint">{t("servers.running")}</p>{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
	{#if working}<p class="hint" role="status">{t("servers.working")}</p>{/if}
	{#if querying}<p class="hint" role="status">{t("servers.querying")}</p>{/if}
	<div class="search-field">
		<Icon name="ui:search" />
		<input
			type="search"
			bind:value={search}
			oninput={() => (page = 0)}
			placeholder={t("servers.search")}
			aria-label={t("servers.search")}
		/>
	</div>
	{#if loading && !list.servers.length}
		<p class="empty" role="status">{t("servers.loading")}</p>
	{:else if !filtered.length}
		<p class="empty">
			{t(list.servers.length ? "servers.noResults" : "servers.empty")}
		</p>
	{:else}
		<div class="browser" aria-busy={loading}>
			<div class="server-list">
				{#each visible as server (server.key)}
					{@const status = statuses[server.index]}
					{@const icon = status?.icon ?? icons[server.index]}
					<button
						type="button"
						class="server-row"
						class:selected={selectedIndex === server.index}
						aria-pressed={selectedIndex === server.index}
						onclick={() => (selectedIndex = server.index)}
						ondblclick={() => void connect(server)}
					>
						<div class="server-icon">
							{#if icon}<img
									src={icon}
									alt=""
									loading="lazy"
								/>{:else}<Icon
									name="instance:servers"
									size={30}
								/>{/if}
						</div>
						<div class="server-label">
							<strong>{server.name}</strong><span
								>{server.address}</span
							><small class:online={status?.online}
								>{statusLabel(status)}{#if status?.online}
									· {playerCount(status)} · {status.ping} ms{/if}</small
							>
						</div>
					</button>
				{/each}
				{#if pages > 1}<div class="pagination">
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
					</div>{/if}
			</div>
			{#if selected}
				<aside class="details">
					<h3>{selected.name}</h3>
					{#if selectedStatus?.motd}<p class="motd">
							{selectedStatus.motd}
						</p>{/if}
					<dl>
						<dt>{t("servers.address")}</dt>
						<dd>{selected.address}</dd>
						<dt>{t("servers.status")}</dt>
						<dd>{statusLabel(selectedStatus)}</dd>
						{#if selectedStatus?.online}
							<dt>{t("servers.players")}</dt>
							<dd>{playerCount(selectedStatus)}</dd>
							<dt>{t("servers.ping")}</dt>
							<dd>{selectedStatus.ping} ms</dd>
							{#if selectedStatus.version}<dt>
									{t("servers.version")}
								</dt>
								<dd>{selectedStatus.version}</dd>{/if}
						{/if}
						<dt>{t("servers.resourcePacks")}</dt>
						<dd>{t(`servers.${selected.resourcePolicy}`)}</dd>
					</dl>
					<div class="detail-actions">
						<button
							type="button"
							class="connect"
							disabled={disabled || running || downloading}
							onclick={() => selected && void connect(selected)}
							><Icon name="ui:play" />{t(
								"servers.connect",
							)}</button
						>
						<button
							type="button"
							disabled={disabled || running}
							onclick={() => showModal("edit")}
							>{t("servers.edit")}</button
						>
						<button
							type="button"
							disabled={disabled ||
								running ||
								selected.index === 0}
							onclick={() =>
								selected &&
								void perform({
									type: "move",
									index: selected.index,
									direction: "up",
								})}>{t("servers.moveUp")}</button
						>
						<button
							type="button"
							disabled={disabled ||
								running ||
								selected.index === list.servers.length - 1}
							onclick={() =>
								selected &&
								void perform({
									type: "move",
									index: selected.index,
									direction: "down",
								})}>{t("servers.moveDown")}</button
						>
						<button
							type="button"
							class="danger"
							disabled={disabled || running}
							onclick={() => showModal("delete")}
							>{t("servers.delete")}</button
						>
					</div>
				</aside>
			{/if}
		</div>
	{/if}
</section>

<ModalBase
	bind:open={modalOpen}
	title={t(`servers.${modalAction}`)}
	width="460px"
>
	<form
		onsubmit={(event) => {
			event.preventDefault();
			submit();
		}}
	>
		{#if modalAction === "delete"}<p>
				{t("servers.deleteConfirm", { name: draftName })}
			</p>
		{:else}
			<label for="server-name">{t("servers.name")}</label><input
				id="server-name"
				bind:value={draftName}
				required
				maxlength={256}
				disabled={working || running}
			/>
			<label for="server-address">{t("servers.address")}</label><input
				id="server-address"
				bind:value={draftAddress}
				required
				maxlength={255}
				placeholder="play.example.com:25565"
				disabled={working || running}
				spellcheck="false"
			/>
			<p class="hint">{t("servers.addressHint")}</p>
			<div
				class="policy-field"
				role="group"
				aria-labelledby="server-policy-label"
			>
				<span id="server-policy-label"
					>{t("servers.resourcePacks")}</span
				>
				<Select
					bind:value={draftPolicy}
					disabled={working || running}
					options={[
						{ value: "prompt", label: t("servers.prompt") },
						{ value: "enabled", label: t("servers.enabled") },
						{ value: "disabled", label: t("servers.disabled") },
					]}
				/>
			</div>
		{/if}
		{#if error}<p class="error" role="alert">{error}</p>{/if}
		<div class="modal-actions">
			<button
				type="button"
				disabled={working}
				onclick={() => (modalOpen = false)}>{t("common.cancel")}</button
			><button
				type="submit"
				class:danger={modalAction === "delete"}
				disabled={disabled ||
					running ||
					(modalAction !== "delete" &&
						(!draftName.trim() || !draftAddress.trim()))}
				>{t(
					modalAction === "delete"
						? "servers.delete"
						: "servers.save",
				)}</button
			>
		</div>
	</form>
</ModalBase>

<style>
	.servers {
		display: flex;
		flex-direction: column;
		gap: 16px;
		color: var(--text-primary);
	}
	.toolbar,
	.actions,
	h2,
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
	.actions button,
	.connect {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
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
	.search-field {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 0 12px;
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		color: var(--text-tertiary);
	}
	.search-field:focus-within {
		border-color: var(--accent-primary);
	}
	.search-field input {
		width: 100%;
		min-width: 0;
		border: none;
		background: transparent;
		outline: none;
	}
	.browser {
		display: grid;
		grid-template-columns: minmax(240px, 1fr) minmax(240px, 320px);
		gap: 20px;
		align-items: start;
	}
	.server-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-width: 0;
	}
	.server-row {
		display: flex;
		align-items: center;
		gap: 12px;
		text-align: left;
		padding: 12px;
		min-height: 84px;
	}
	.server-row.selected {
		border-color: var(--accent-primary);
		background: var(--bg-item-active);
	}
	.server-icon {
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
	.server-icon img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		image-rendering: pixelated;
	}
	.server-label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
		flex: 1;
	}
	.server-label strong,
	.server-label span,
	.server-label small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.server-label span,
	.server-label small,
	dt,
	.hint {
		color: var(--text-secondary);
		font-size: 0.8rem;
	}
	.server-label .online {
		color: var(--accent-primary);
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
	.motd {
		white-space: pre-wrap;
		font-size: 0.85rem;
		line-height: 1.5;
	}
	.detail-actions {
		margin-top: 20px;
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}
	.connect {
		grid-column: 1 / -1;
		border-color: var(--accent-primary);
	}
	.danger,
	.error {
		color: var(--error, #ef6464);
	}
	.error {
		overflow-wrap: anywhere;
	}
	.hint,
	.error {
		margin: 0;
		line-height: 1.5;
	}
	.empty {
		padding: 56px 20px;
		text-align: center;
		color: var(--text-secondary);
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
	label,
	.policy-field {
		font-size: 0.85rem;
	}
	.policy-field {
		display: flex;
		flex-direction: column;
		gap: 14px;
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
