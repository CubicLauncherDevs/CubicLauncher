<script lang="ts">
	import { onDestroy } from "svelte";
	import { t } from "$lib/i18n";
	import { openUrl } from "$lib/api/cubicApi";
	import Icon from "$lib/icons/Icon.svelte";
	import type { MarketDetailState } from "$lib/state/marketState.svelte";
	import type { ContentType, MarketProject } from "$lib/types/market";
	import type {
		CurseForgeProject,
		ModrinthProjectFull,
	} from "$lib/types/types";

	let {
		project,
		detail,
		contentType,
	}: {
		project: MarketProject;
		detail: MarketDetailState;
		contentType: ContentType;
	} = $props();

	const modrinth = $derived(
		project.source === "modrinth"
			? (detail.fullProject as ModrinthProjectFull | undefined)
			: undefined,
	);
	const curseforge = $derived(
		project.source === "curseforge"
			? (detail.fullProject as CurseForgeProject | undefined)
			: undefined,
	);
	const sourceName = $derived(
		project.source === "modrinth"
			? "Modrinth"
			: project.source === "curseforge"
				? "CurseForge"
				: t("market.filter.sourceLocal"),
	);
	const projectUrl = $derived.by(() => {
		const slug = detail.fullProject?.slug ?? project.slug;
		if (project.source === "curseforge") {
			return (
				curseforge?.links?.websiteUrl ??
				(slug
					? `https://www.curseforge.com/minecraft/${contentType === "resourcepacks" ? "texture-packs" : contentType === "shaderpacks" ? "shaders" : "mc-mods"}/${slug}`
					: null)
			);
		}
		if (project.source !== "modrinth") return null;
		const id = slug ?? project.modrinthProjectId ?? project.id;
		return `https://modrinth.com/${contentType === "resourcepacks" ? "resourcepack" : contentType === "shaderpacks" ? "shader" : "mod"}/${id}`;
	});
	const loaderIcons: Record<string, string> = {
		fabric: "brand:fabric",
		forge: "brand:forge",
		neoforge: "brand:neoforged",
		quilt: "brand:quilt",
		optifine: "brand:optifine",
		minecraft: "brand:vanilla",
	};
	const loaderNames: Record<string, string> = {
		fabric: "Fabric",
		forge: "Forge",
		neoforge: "NeoForge",
		quilt: "Quilt",
		optifine: "OptiFine",
		iris: "Iris",
		minecraft: "Minecraft",
	};
	const loaders = $derived([
		...new Set(
			modrinth?.loaders ??
				detail.versions.flatMap((version) => version.loaders),
		),
	]);
	const categories = $derived(
		modrinth?.categories.map((category) => {
			const key = `market.categories.${category}`;
			const translated = t(key);
			return {
				id: category,
				label:
					translated === key
						? category.replaceAll("-", " ")
						: translated,
			};
		}) ??
			curseforge?.categories.map((category) => ({
				id: String(category.id),
				label: category.name,
			})) ??
			[],
	);
	const links = $derived(
		[
			{
				label: "Discord",
				icon: "brand:discord",
				url: modrinth?.discord_url,
			},
			{
				label: t("market.detail.issues"),
				icon: "ui:flag",
				url: modrinth?.issues_url ?? curseforge?.links?.issuesUrl,
			},
			{
				label: t("market.detail.sourceCode"),
				icon: "instance:code",
				url: modrinth?.source_url ?? curseforge?.links?.sourceUrl,
			},
			{
				label: "Wiki",
				icon: "instance:resources",
				url: modrinth?.wiki_url ?? curseforge?.links?.wikiUrl,
			},
		].filter((link) => link.url),
	);

	function sideLabel(value: string | undefined): string | null {
		switch (value) {
			case "required":
				return t("market.detail.sideRequired");
			case "optional":
				return t("market.detail.sideOptional");
			case "unsupported":
				return t("market.detail.sideUnsupported");
			case "unknown":
				return t("market.detail.sideUnknown");
			default:
				return value || null;
		}
	}
	const technicalRows = $derived([
		{
			id: "client",
			label: t("market.detail.clientSide"),
			icon: "instance:screenshots",
			value: sideLabel(modrinth?.client_side),
		},
		{
			id: "server",
			label: t("market.detail.serverSide"),
			icon: "instance:servers",
			value: sideLabel(modrinth?.server_side),
		},
		{
			id: "license",
			label: t("market.detail.license"),
			icon: "instance:resources",
			value: modrinth?.license?.id || modrinth?.license?.name || null,
			name: modrinth?.license?.name,
			url: modrinth?.license?.url,
		},
		{
			id: "project",
			label: t("market.detail.projectId"),
			icon: "instance:code",
			value: String(
				detail.fullProject?.id ??
					project.modrinthProjectId ??
					project.curseforgeProjectId ??
					project.id,
			),
		},
	]);
	let copied = $state<string | null>(null);
	let copyError = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | undefined;
	let disposed = false;
	onDestroy(() => {
		disposed = true;
		clearTimeout(copyTimer);
	});
	async function copyValue(id: string, value: string) {
		try {
			await navigator.clipboard.writeText(value);
			if (disposed) return;
			clearTimeout(copyTimer);
			copyError = false;
			copied = id;
			copyTimer = setTimeout(() => {
				copied = null;
			}, 2000);
		} catch {
			if (!disposed) copyError = true;
		}
	}
</script>

<div class="project-info">
	<section class="info-section">
		<h3>{t("market.detail.modSource")}</h3>
		{#if projectUrl}
			<button
				type="button"
				class="source-link"
				onclick={() => openUrl(projectUrl!)}
			>
				<Icon name={`brand:${project.source}`} size={26} />
				{sourceName}
				<Icon name="instance:external-link" size={12} />
			</button>
		{:else}
			<span class="source-link"
				><Icon name="instance:folder" size={24} />{sourceName}</span
			>
		{/if}
	</section>
	{#if loaders.length > 0}
		<section class="info-section">
			<h3>{t("market.detail.modLoaders")}</h3>
			<div class="loaders">
				{#each loaders as loader (loader)}
					<span class="loader" title={loaderNames[loader] ?? loader}>
						<Icon
							name={loaderIcons[loader] ?? "instance:puzzle"}
							size={28}
						/>
						<span>{loaderNames[loader] ?? loader}</span>
					</span>
				{/each}
			</div>
		</section>
	{/if}
	{#if categories.length > 0}
		<section class="info-section">
			<h3>{t("market.detail.categories")}</h3>
			<div class="categories">
				{#each categories as category (category.id)}
					<span class="category">
						<Icon
							name={category.id === "optimization"
								? "ui:performance"
								: "instance:puzzle"}
							size={16}
						/>
						{category.label}
					</span>
				{/each}
			</div>
		</section>
	{/if}
	{#if links.length > 0}
		<section class="info-section">
			<h3>{t("market.detail.externalResources")}</h3>
			<div class="external-links">
				{#each links as link (link.label)}
					<button
						type="button"
						class="external-link"
						onclick={() => openUrl(link.url!)}
					>
						<Icon name={link.icon} size={16} />
						<span>{link.label}</span>
						<Icon name="instance:external-link" size={12} />
					</button>
				{/each}
			</div>
		</section>
	{/if}
	{#if project.source !== "local"}
		<section class="info-section">
			<h3>{t("market.detail.technicalInfo")}</h3>
			<dl class="technical-info">
				{#each technicalRows as row (row.id)}
					<div class="technical-card">
						<dt><Icon name={row.icon} size={14} />{row.label}</dt>
						<dd>
							{#if row.url}
								<button
									type="button"
									class="license-link"
									title={row.name ?? row.value ?? ""}
									onclick={() => openUrl(row.url!)}
									>{row.name ?? row.value}</button
								>
							{:else}
								<span
									class="technical-value"
									title={row.name ?? row.value ?? ""}
									>{row.name ??
										row.value ??
										t("market.detail.notProvided")}</span
								>
							{/if}
							{#if row.value}
								<button
									type="button"
									class="copy-value"
									title={row.value}
									aria-label={t("market.detail.copyValue", {
										label: row.label,
										value: row.value,
									})}
									onclick={() =>
										copyValue(row.id, row.value!)}
								>
									<span>{row.value}</span>
									<Icon
										name={copied === row.id
											? "ui:check"
											: "ui:copy"}
										size={12}
									/>
								</button>
							{/if}
						</dd>
					</div>
				{/each}
			</dl>
			<span class="copy-status" class:copy-error={copyError} role="status"
				>{copyError
					? t("market.detail.copyFailed")
					: copied
						? t("market.detail.copied")
						: ""}</span
			>
		</section>
	{/if}
</div>

<style>
	.project-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.info-section {
		padding: 16px 0;
		border-top: 1px solid var(--border);
	}
	.info-section:last-child {
		padding-bottom: 0;
	}
	h3 {
		margin: 0 0 12px;
		font-size: 0.8rem;
		font-weight: 500;
		color: var(--text-secondary);
	}
	button {
		font: inherit;
		cursor: pointer;
		color: inherit;
	}
	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 3px;
	}
	.source-link {
		display: inline-flex;
		align-items: center;
		gap: 10px;
		padding: 6px 8px;
		border: 0;
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-primary);
		font-size: 0.8rem;
	}
	.source-link:hover,
	.external-link:hover {
		background: var(--surface-hover);
	}
	.loaders,
	.categories {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
	.loader {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		min-width: 42px;
		color: var(--text-secondary);
		font-size: 0.65rem;
	}
	.category {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 5px 8px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		color: var(--text-primary);
		text-transform: capitalize;
		overflow-wrap: anywhere;
	}
	.external-links {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.external-link {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px;
		border: 0;
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-secondary);
		text-align: left;
		font-size: 0.8rem;
	}
	.external-link > span {
		flex: 1;
		color: var(--text-primary);
	}
	.technical-info {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin: 0;
	}
	.technical-card {
		min-width: 0;
		padding: 8px;
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
	}
	dt {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 5px;
		color: var(--text-secondary);
		font-size: 0.6rem;
		font-weight: 600;
		letter-spacing: 0.5px;
		text-transform: uppercase;
	}
	dd {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		margin: 0;
		color: var(--text-primary);
		font-size: 0.78rem;
	}
	.technical-value,
	.license-link {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.license-link {
		padding: 0;
		border: 0;
		background: transparent;
		text-decoration: underline;
		text-underline-offset: 3px;
	}
	.copy-value {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
		max-width: 68%;
		padding: 4px 8px;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: transparent;
		font-size: 0.65rem;
	}
	.copy-value span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.copy-value:hover {
		background: var(--surface-hover);
		border-color: var(--accent);
	}
	.copy-status {
		display: block;
		color: var(--color-success);
		font-size: 0.7rem;
	}
	.copy-status:not(:empty) {
		margin-top: 8px;
	}
	.copy-error {
		color: var(--color-error);
	}
</style>
