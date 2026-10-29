<script lang="ts">
	import { onDestroy, untrack } from "svelte";
	import { SvelteMap, SvelteSet } from "svelte/reactivity";
	import { t } from "$lib/i18n";
	import { openUrl } from "$lib/api/cubicApi";
	import {
		CATALOG_CATEGORIES,
		MOSAICARY_REPO,
		MOSAICARY_SITE,
		CATALOG_TAGS_MOST,
		NO_CATALOG_FILTERS,
		fetchCatalogPage,
		fetchCatalogTags,
		keepLoaded,
		loadedFor,
		type CatalogArms,
		type CatalogTag,
		type CatalogCategory,
		type CatalogFilters,
		type CatalogSkin,
		type CatalogSort,
	} from "$lib/api/mosaicarySkins";
	import Icon from "$lib/icons/Icon.svelte";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import Select from "$lib/components/layout/Select.svelte";
	import TagPicker from "./TagPicker.svelte";
	import Tooltip from "$lib/components/ui/Tooltip.svelte";
	import { shouldReduceAnimations } from "$lib/utils/animations";

	interface Props {
		open?: boolean;
		/** La skin elegida: queda pendiente en la vista previa hasta guardar. */
		onPick: (skin: CatalogSkin) => void;
	}

	let { open = $bindable(false), onPick }: Props = $props();

	let filters = $state<CatalogFilters>({ ...NO_CATALOG_FILTERS });
	// Sin proxy profundo: la lista solo se reemplaza entera, nunca se cambia por dentro, y
	// así cientos de tarjetas no pasan por el seguimiento de Svelte ni se copian al guardarlas.
	let skins = $state.raw<CatalogSkin[]>([]);
	let page = $state(0);
	let pages = $state(0);
	let searching = $state(false);
	let loadingMore = $state(false);
	let failed = $state(false);
	/** Si el final de la lista está a menos de una pantalla de verse. */
	let nearEnd = $state(false);
	// No reactivos: la generación de la búsqueda en curso (una respuesta vieja no pisa una
	// nueva), la petición que se cancela al buscar de nuevo y la espera mientras se escribe.
	let generation = 0;
	let request: AbortController | undefined;
	let typing: ReturnType<typeof setTimeout> | undefined;
	let loadedOnce = false;

	// El catálogo se monta dentro del perfil, que crea su propio contexto de
	// posicionamiento: llevado al body, la ventana cubre toda la app y no queda
	// recortada bajo la barra lateral.
	function portal(el: HTMLElement) {
		document.body.appendChild(el);
		return {
			destroy() {
				el.remove();
			},
		};
	}

	// Lleva al final de la lista hasta el borde del área que se desplaza, con una pantalla
	// de margen: la página siguiente se pide antes de que se acabe lo que se ve.
	function watchEnd(node: HTMLElement) {
		const observer = new IntersectionObserver(
			(seen) => {
				nearEnd = seen.some((one) => one.isIntersecting);
			},
			{
				root: node.closest(".modal-body"),
				rootMargin: "0px 0px 100% 0px",
			},
		);
		observer.observe(node);
		return {
			destroy() {
				observer.disconnect();
				nearEnd = false;
			},
		};
	}

	async function load(reset: boolean) {
		if (!reset && (searching || loadingMore)) return;
		const mine = ++generation;
		request?.abort();
		const asked = $state.snapshot(filters);
		const cached = reset ? loadedFor(asked) : null;
		if (cached) {
			blockHeights.clear();
			nearBlocks.clear();
			({ skins, page, pages } = cached);
			searching = false;
			loadingMore = false;
			failed = false;
			return;
		}
		const controller = new AbortController();
		request = controller;
		if (reset) searching = true;
		else loadingMore = true;
		failed = false;
		try {
			const result = await fetchCatalogPage(
				asked,
				reset ? 1 : page + 1,
				controller.signal,
			);
			if (mine !== generation) return;
			if (reset) {
				blockHeights.clear();
				nearBlocks.clear();
				skins = result.skins;
			} else {
				const known = new Set(skins.map((one) => one.id));
				skins = [
					...skins,
					...result.skins.filter((one) => !known.has(one.id)),
				];
			}
			page = result.page;
			pages = result.pages;
			keepLoaded(asked, { skins, page, pages });
		} catch {
			if (mine === generation && !controller.signal.aborted)
				failed = true;
		} finally {
			if (mine === generation) {
				searching = false;
				loadingMore = false;
			}
		}
	}

	// La primera página se pide al abrir la ventana por primera vez, no antes: quien no
	// abre el catálogo no hace ninguna petición. Lo cargado se conserva al cerrarla.
	$effect(() => {
		if (open && !loadedOnce) {
			loadedOnce = true;
			untrack(() => void load(true));
		}
	});

	// Al acercarse al final, la página siguiente. También justo después de cargar una, si
	// el final sigue a la vista: el observador no avisa de lo que no cambia.
	$effect(() => {
		if (nearEnd && page < pages && !searching && !loadingMore && !failed)
			untrack(() => void load(false));
	});

	function search() {
		clearTimeout(typing);
		void load(true);
	}

	function handleInput() {
		clearTimeout(typing);
		typing = setTimeout(search, 350);
	}

	function setFilter<K extends Exclude<keyof CatalogFilters, "query">>(
		key: K,
		value: CatalogFilters[K],
	) {
		if (filters[key] === value) return;
		filters[key] = value;
		search();
	}

	function pick(skin: CatalogSkin) {
		onPick(skin);
		open = false;
	}

	/* Etiquetas. */

	let tagList = $state<CatalogTag[]>([]);
	let tagsAsked = false;

	// Se piden con la primera página: quien no abre el catálogo no pide nada.
	$effect(() => {
		if (!open || tagsAsked) return;
		tagsAsked = true;
		fetchCatalogTags().then(
			(got) => (tagList = got),
			() => (tagsAsked = false),
		);
	});

	function toggleTag(tag: string) {
		const chosen = filters.tags.includes(tag)
			? filters.tags.filter((one) => one !== tag)
			: [...filters.tags, tag].slice(-CATALOG_TAGS_MOST);
		filters.tags = chosen;
		search();
	}

	function clearTags() {
		if (!filters.tags.length) return;
		filters.tags = [];
		search();
	}

	function setTagsAll(all: boolean) {
		if (filters.tagsAll === all) return;
		filters.tagsAll = all;
		if (filters.tags.length > 1) search();
	}

	/* La explicación de los ojos vivos, en una burbuja junto a su «?». */

	let eyesHintOpen = $state(false);
	let eyesHintAt = $state({ x: 0, y: 0 });
	function showEyesHint(event: PointerEvent) {
		const box = (
			event.currentTarget as HTMLElement
		).getBoundingClientRect();
		eyesHintAt = { x: box.left + box.width / 2, y: box.bottom };
		eyesHintOpen = true;
	}

	/* Tarjetas por fila: como en mosaicary.com, a gusto de quien mira y guardado aquí. */

	const PER_ROW_KEY = "mosaicary-catalog-per-row";
	const PER_ROW_CHOICES = [3, 4, 5, 6] as const;
	function storedPerRow(): number {
		try {
			const kept = Number(localStorage.getItem(PER_ROW_KEY));
			return (PER_ROW_CHOICES as readonly number[]).includes(kept)
				? kept
				: 4;
		} catch {
			return 4;
		}
	}
	let perRow = $state(storedPerRow());
	function setPerRow(value: string) {
		perRow = Number(value) || 4;
		// Con otras tarjetas por fila los bloques son otros: sus alturas ya no valen.
		blockHeights.clear();
		try {
			localStorage.setItem(PER_ROW_KEY, String(perRow));
		} catch {
			// Sin almacenamiento la elección dura lo que la ventana.
		}
	}
	const perRowOptions = $derived(
		PER_ROW_CHOICES.map((one) => ({
			value: String(one),
			label: t("userMenu.skinCape.catalog.perRowValue", { count: one }),
		})),
	);

	/* La lista en bloques de unas filas. */

	/** Las mismas cuatro filas que reserva `contain-intrinsic-size` en `.catalog-block`. */
	const ROWS_PER_BLOCK = 4;
	// Cada bloque es su propia rejilla, y el navegador se salta la maquetación y el pintado
	// de los que quedan fuera de la vista (`content-visibility`). En una sola rejilla, cada
	// imagen que llegaba y cada página nueva volvían a maquetar todas las tarjetas: con 800,
	// hasta 120 ms cada vez, y la lista daba tirones al desplazarse.
	const blocks = $derived.by(() => {
		const size = perRow * ROWS_PER_BLOCK;
		const out: CatalogSkin[][] = [];
		for (let at = 0; at < skins.length; at += size)
			out.push(skins.slice(at, at + size));
		return out;
	});

	// Solo los bloques a menos de una pantalla de la vista llevan sus tarjetas; los demás
	// quedan vacíos con la altura que tenían, o la que tendrían, y la lista no se mueve.
	// Con todas puestas, abrir otra vez el catálogo con 1600 tarjetas cargadas las creaba
	// de golpe: un tercio de segundo con la ventana parada.
	const nearBlocks = new SvelteSet<number>();
	/** La altura de cada bloque al alejarse, para que vacío ocupe lo mismo. */
	const blockHeights = new SvelteMap<number, number>();

	let blockObserver: IntersectionObserver | null = null;
	let blocksWatched = 0;
	const blockAt = new WeakMap<Element, number>();

	function observerFor(root: Element): IntersectionObserver {
		if (!blockObserver) {
			blockObserver = new IntersectionObserver(
				(seen) => {
					for (const one of seen) {
						const at = blockAt.get(one.target);
						if (at === undefined) continue;
						if (one.isIntersecting) {
							nearBlocks.add(at);
						} else if (nearBlocks.has(at)) {
							blockHeights.set(
								at,
								(
									one.target as HTMLElement
								).getBoundingClientRect().height,
							);
							nearBlocks.delete(at);
						}
					}
				},
				{ root, rootMargin: "100% 0px" },
			);
		}
		return blockObserver;
	}

	function watchBlock(node: HTMLElement, at: number) {
		const observer = observerFor(node.closest(".modal-body") ?? node);
		let current = at;
		blockAt.set(node, at);
		blocksWatched++;
		observer.observe(node);
		return {
			update(next: number) {
				current = next;
				blockAt.set(node, next);
			},
			destroy() {
				observer.unobserve(node);
				blockAt.delete(node);
				nearBlocks.delete(current);
				blockHeights.delete(current);
				blocksWatched--;
				if (blocksWatched <= 0) {
					observer.disconnect();
					blockObserver = null;
				}
			},
		};
	}

	function releaseImage(img: HTMLImageElement) {
		return {
			destroy() {
				img.removeAttribute("src");
			},
		};
	}

	function farHeight(at: number): string | null {
		const height = blockHeights.get(at);
		return height ? `${height}px` : null;
	}

	/* Vuelta a los filtros, cuando la lista ya los dejó atrás. */

	let filtersOutOfView = $state(false);
	let scroller: HTMLElement | null = null;

	// Con un observador y no midiendo en cada evento de desplazamiento: medir obliga al
	// navegador a maquetar en mitad del desplazamiento si algo cambió, como una imagen que llega.
	function watchScroll(node: HTMLElement) {
		scroller = node.closest(".modal-body");
		const observer = new IntersectionObserver(
			(seen) => {
				const last = seen[seen.length - 1]!;
				// Fuera de la vista por arriba, no por abajo.
				filtersOutOfView =
					!last.isIntersecting &&
					last.boundingClientRect.bottom <=
						(last.rootBounds?.top ?? 0);
			},
			{ root: scroller },
		);
		observer.observe(node);
		return {
			destroy() {
				observer.disconnect();
				scroller = null;
				filtersOutOfView = false;
			},
		};
	}

	function backToFilters() {
		scroller?.scrollTo({
			top: 0,
			behavior: shouldReduceAnimations() ? "auto" : "smooth",
		});
	}

	onDestroy(() => {
		generation++;
		request?.abort();
		clearTimeout(typing);
		blockObserver?.disconnect();
		blockObserver = null;
		blocksWatched = 0;
		nearBlocks.clear();
		blockHeights.clear();
	});

	const hasMore = $derived(page < pages);
	/** Una página siguiente que no llegó: se dice en una línea, con su reintento. */
	const moreFailed = $derived(failed && skins.length > 0);

	const armsOptions = $derived([
		{ value: "any", label: t("userMenu.skinCape.catalog.allModels") },
		{ value: "classic", label: t("userMenu.skinCape.classic") },
		{ value: "slim", label: t("userMenu.skinCape.slim") },
	]);

	const sortOptions = $derived([
		{ value: "new", label: t("userMenu.skinCape.catalog.sortNew") },
		{ value: "popular", label: t("userMenu.skinCape.catalog.sortPopular") },
		{
			value: "downloads",
			label: t("userMenu.skinCape.catalog.sortDownloads"),
		},
	]);

	const eyesOptions = $derived([
		{ value: "any", label: t("userMenu.skinCape.catalog.eyesAny") },
		{ value: "alive", label: t("userMenu.skinCape.catalog.eyesAlive") },
	]);

	const categoryOptions = $derived([
		{ value: "any", label: t("userMenu.skinCape.catalog.allCategories") },
		...CATALOG_CATEGORIES.map((one) => ({
			value: one,
			label: t(`userMenu.skinCape.catalog.categories.${one}`),
		})),
	]);
</script>

<div use:portal>
	<ModalBase
		bind:open
		title={t("userMenu.skinCape.catalog.title")}
		width="760px"
		scrollBody
	>
		<p class="catalog-credit">
			{t("userMenu.skinCape.catalog.by")}
			<button
				type="button"
				class="link-btn"
				onclick={() => openUrl(MOSAICARY_SITE)}
				><Icon name="brand:mosaicary" size={14} />mosaicary.com</button
			>
			·
			<button
				type="button"
				class="link-btn"
				onclick={() => openUrl(MOSAICARY_REPO)}
				><Icon name="brand:github" size={13} />GitHub</button
			>
		</p>

		<div class="filter-bar" use:watchScroll>
			<div class="filter-group wide">
				<span class="filter-label"
					>{t("userMenu.skinCape.catalog.searchLabel")}</span
				>
				<div class="search-box">
					<span class="search-icon"
						><Icon name="ui:search" size={14} /></span
					>
					<input
						type="search"
						class="text-input"
						placeholder={t("userMenu.skinCape.catalog.search")}
						bind:value={filters.query}
						oninput={handleInput}
						onkeydown={(e) => e.key === "Enter" && search()}
						maxlength="100"
					/>
				</div>
			</div>
			<div class="filter-group">
				<span class="filter-label"
					>{t("userMenu.skinCape.catalog.model")}</span
				>
				<Select
					compact
					value={filters.arms}
					options={armsOptions}
					placeholder={t("userMenu.skinCape.catalog.model")}
					onchange={(value) =>
						setFilter("arms", value as CatalogArms)}
				/>
			</div>
			<div class="filter-group">
				<span class="filter-label"
					>{t("userMenu.skinCape.catalog.order")}</span
				>
				<Select
					compact
					value={filters.sort}
					options={sortOptions}
					placeholder={t("userMenu.skinCape.catalog.order")}
					onchange={(value) =>
						setFilter("sort", value as CatalogSort)}
				/>
			</div>
			<div class="filter-group">
				<span class="filter-label">
					{t("userMenu.skinCape.catalog.eyes")}
					<!-- Lo que hacen los ojos vivos, al pasar por encima: no dentro de la lista. -->
					<span
						class="hint-mark"
						role="note"
						aria-label={t("userMenu.skinCape.catalog.eyesHint")}
						onpointerenter={showEyesHint}
						onpointerleave={() => (eyesHintOpen = false)}>?</span
					>
				</span>
				<Select
					compact
					value={filters.eyes ? "alive" : "any"}
					options={eyesOptions}
					placeholder={t("userMenu.skinCape.catalog.eyes")}
					onchange={(value) => setFilter("eyes", value === "alive")}
				/>
			</div>
			<div class="filter-group">
				<span class="filter-label"
					>{t("userMenu.skinCape.catalog.category")}</span
				>
				<Select
					compact
					value={filters.category}
					options={categoryOptions}
					placeholder={t("userMenu.skinCape.catalog.category")}
					onchange={(value) =>
						setFilter("category", value as CatalogCategory)}
				/>
			</div>
			<div class="filter-group">
				<span class="filter-label"
					>{t("userMenu.skinCape.catalog.tags")}</span
				>
				<TagPicker
					tags={tagList}
					chosen={filters.tags}
					most={CATALOG_TAGS_MOST}
					all={filters.tagsAll}
					onToggle={toggleTag}
					onClear={clearTags}
					onAll={setTagsAll}
				/>
			</div>
			<div class="filter-group">
				<span class="filter-label"
					>{t("userMenu.skinCape.catalog.perRow")}</span
				>
				<Select
					compact
					value={String(perRow)}
					options={perRowOptions}
					placeholder={t("userMenu.skinCape.catalog.perRow")}
					onchange={setPerRow}
				/>
			</div>
		</div>
		<Tooltip
			open={eyesHintOpen}
			x={eyesHintAt.x}
			y={eyesHintAt.y}
			placement="bottom"
			>{t("userMenu.skinCape.catalog.eyesHint")}</Tooltip
		>

		{#if searching && skins.length === 0}
			<div class="catalog-state">
				<span class="spinner"></span>
			</div>
		{:else if failed && skins.length === 0}
			<div class="catalog-state">
				<span>{t("userMenu.skinCape.catalog.failed")}</span>
				<button type="button" class="btn-secondary" onclick={search}
					>{t("userMenu.skinCape.catalog.retry")}</button
				>
			</div>
		{:else if skins.length === 0}
			<p class="catalog-state">{t("userMenu.skinCape.catalog.empty")}</p>
		{:else}
			<div
				class="catalog-grid"
				class:dimmed={searching}
				role="list"
				style:--per-row={perRow}
			>
				{#each blocks as block, at (at)}
					{@const near = nearBlocks.has(at)}
					<div
						class="catalog-block"
						class:far={!near}
						role="none"
						style:--rows={Math.ceil(block.length / perRow)}
						style:height={near ? null : farHeight(at)}
						use:watchBlock={at}
					>
						{#if near}
							{#each block as skin (skin.id)}
								<div class="catalog-item" role="listitem">
									<button
										type="button"
										class="catalog-pick"
										onclick={() => pick(skin)}
										title={t(
											"userMenu.skinCape.catalog.use",
										)}
									>
										<!-- La skin puesta y girada, dibujada por Mosaicary como en sus
							     tarjetas: una imagen en caché, nada de 3D por tarjeta. Con ojos
							     vivos, un APNG que solo cambia los píxeles de los ojos; el
							     navegador no lo anima fuera de la vista. -->
										<img
											src={skin.live &&
											!shouldReduceAnimations()
												? skin.live
												: skin.picture}
											alt={skin.name}
											width="256"
											height="256"
											decoding="async"
											fetchpriority="low"
											draggable="false"
											use:releaseImage
										/>
										<span
											class="catalog-name"
											title={skin.name}>{skin.name}</span
										>
										{#if skin.credit}
											<span
												class="catalog-by"
												title={skin.credit}
											>
												{skin.worn
													? t(
															"userMenu.skinCape.catalog.wornBy",
															{
																name: skin.credit,
															},
														)
													: skin.credit}
											</span>
										{/if}
									</button>
									<button
										type="button"
										class="catalog-page"
										onclick={() => openUrl(skin.page)}
										title={t(
											"userMenu.skinCape.catalog.page",
										)}
										aria-label={t(
											"userMenu.skinCape.catalog.page",
										)}
									>
										<Icon
											name="instance:external-link"
											size={12}
										/>
									</button>
								</div>
							{/each}
						{/if}
					</div>
				{/each}
			</div>
			{#if moreFailed}
				<p class="catalog-more">
					{t("userMenu.skinCape.catalog.moreFailed")}
					<button
						type="button"
						class="link-btn"
						onclick={() => load(false)}
						>{t("userMenu.skinCape.catalog.retry")}</button
					>
				</p>
			{:else if hasMore}
				<!-- La página siguiente llega sola al acercarse aquí. -->
				<div class="catalog-more" use:watchEnd>
					{#if loadingMore}<span class="spinner"></span>{/if}
				</div>
			{/if}
		{/if}
		{#if filtersOutOfView}
			<!-- Pegado al borde inferior de lo que se ve mientras la lista sigue. -->
			<button
				type="button"
				class="to-filters"
				onclick={backToFilters}
				title={t("userMenu.skinCape.catalog.toFilters")}
			>
				<Icon
					name="ui:chevron-down"
					size={14}
					style="transform: rotate(180deg)"
				/>
				{t("userMenu.skinCape.catalog.toFilters")}
			</button>
		{/if}
	</ModalBase>
</div>

<style>
	.catalog-credit {
		margin: 0;
		font-size: 0.75rem;
		color: var(--text-secondary);
	}

	.link-btn {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		vertical-align: middle;
		background: none;
		border: none;
		padding: 0;
		font: inherit;
		color: var(--accent);
		cursor: pointer;
	}

	.link-btn:hover {
		text-decoration: underline;
	}

	.search-box {
		position: relative;
		display: flex;
		align-items: center;
	}

	/* La lupa dentro del campo, a la izquierda del texto. */
	.search-icon {
		position: absolute;
		left: 12px;
		display: inline-flex;
		color: var(--text-muted);
		pointer-events: none;
	}

	.search-box .text-input {
		flex: 1;
		min-width: 0;
		padding-left: 34px;
	}

	.filter-bar {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 10px 12px;
		align-items: end;
	}

	.filter-group.wide {
		grid-column: span 2;
	}

	/* Un valor largo se corta en una línea: el botón no cambia de alto. */
	.filter-bar :global(.select-trigger .select-option-label) {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.hint-mark {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 13px;
		height: 13px;
		margin-left: 4px;
		border: 1px solid var(--text-secondary);
		border-radius: 50%;
		font-size: 0.55rem;
		cursor: help;
		vertical-align: middle;
	}

	.filter-group {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.filter-label {
		font-size: 0.65rem;
		font-weight: 700;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.8px;
	}

	.to-filters {
		position: sticky;
		bottom: 4px;
		align-self: flex-end;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		background: var(--accent);
		color: var(--accent-text);
		border: none;
		border-radius: 20px;
		box-shadow: var(--shadow-lg, 0 6px 16px rgba(0, 0, 0, 0.35));
		font-family: inherit;
		font-size: 0.72rem;
		font-weight: 700;
		cursor: pointer;
		z-index: 2;
	}

	.catalog-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
		margin: 0;
		padding: 32px 12px;
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.catalog-grid {
		display: flex;
		flex-direction: column;
		gap: 10px;
		container-type: inline-size;
		transition: opacity 0.15s ease;
		/* El alto de una tarjeta: la imagen (el ancho de la columna menos el relleno, como
		   mucho 160px) y las dos líneas de texto. */
		--card-height: calc(
			min(
					160px,
					(100cqw - (var(--per-row) - 1) * 10px) / var(--per-row) -
						16px
				) +
				46px
		);
	}

	/* Fuera de la vista no se maqueta ni se pinta. Hasta verse por primera vez ocupa lo que
	   ocuparían sus cuatro filas de tarjetas; después, lo que midió (`auto`). */
	.catalog-block {
		display: grid;
		grid-template-columns: repeat(var(--per-row), minmax(0, 1fr));
		gap: 10px;
		content-visibility: auto;
		contain-intrinsic-size: auto calc(4 * var(--card-height) + 30px);
	}

	/* Lejos de la vista, sin sus tarjetas: la altura que medía o la de sus filas. */
	.catalog-block.far {
		height: calc(
			var(--rows) * var(--card-height) + (var(--rows) - 1) * 10px
		);
	}

	.catalog-grid.dimmed {
		opacity: 0.5;
	}

	.catalog-item {
		position: relative;
		min-width: 0;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		transition: border-color 0.15s ease;
	}

	.catalog-item:hover {
		border-color: var(--accent);
	}

	.catalog-pick {
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		background: none;
		border: none;
		padding: 6px 8px 8px;
		cursor: pointer;
		color: var(--text-primary);
		font-family: inherit;
		min-width: 0;
	}

	.catalog-pick img {
		width: 100%;
		max-width: 160px;
		height: auto;
		aspect-ratio: 1;
		object-fit: contain;
	}

	.catalog-name,
	.catalog-by {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.catalog-name {
		font-size: 0.72rem;
		font-weight: 600;
	}

	.catalog-by {
		font-size: 0.65rem;
		color: var(--text-muted);
	}

	.catalog-page {
		position: absolute;
		top: 6px;
		right: 6px;
		width: 22px;
		height: 22px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--media-overlay);
		border: var(--border-width) solid var(--media-overlay-border);
		color: var(--media-overlay-text);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		opacity: 0;
		transition:
			opacity 0.15s ease,
			color 0.15s ease;
	}

	.catalog-item:hover .catalog-page,
	.catalog-item:focus-within .catalog-page {
		opacity: 1;
	}

	@media (hover: none) {
		.catalog-page {
			opacity: 1;
		}
	}

	.catalog-page:hover {
		color: var(--text-primary);
	}

	.catalog-more {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 6px;
		min-height: 24px;
		margin: 0;
		padding-top: 4px;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.spinner {
		width: 16px;
		height: 16px;
		border: 2px solid var(--border);
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
