import type { SkinModel } from "$lib/utils/skinModel";

// Catálogo de skins de Mosaicary (https://mosaicary.com) por su API abierta: un listado
// JSON con CORS abierto, descrito en https://mosaicary.com/developers. Solo se piden
// skins de 64 px (`size=64`), las únicas que Mojang acepta en una cuenta premium.

export const MOSAICARY_SITE = "https://mosaicary.com";
export const MOSAICARY_REPO = "https://github.com/MopicMP/mosaicary";

/** Skins por página: dos o tres filas en la ventana del catálogo. */
export const CATALOG_PAGE_SIZE = 24;

/** Filtro de brazos del catálogo: Steve, Alex o ambos. */
export type CatalogArms = SkinModel | "any";

/** Las categorías de Mosaicary, en el orden en que las muestra. */
export const CATALOG_CATEGORIES = [
	"girls",
	"boys",
	"characters",
	"creatures",
] as const;
export type CatalogCategory = (typeof CATALOG_CATEGORIES)[number] | "any";

/** Órdenes del catálogo: las más nuevas, las que más gustan, las más descargadas. */
export const CATALOG_SORTS = ["new", "popular", "downloads"] as const;
export type CatalogSort = (typeof CATALOG_SORTS)[number];

export interface CatalogFilters {
	query: string;
	arms: CatalogArms;
	category: CatalogCategory;
	sort: CatalogSort;
	/** Solo skins cuyos ojos se mueven con el paquete de recursos Fresh Moves. */
	eyes: boolean;
	/** Etiquetas: skins con alguna de ellas, o con `tagsAll` con todas. */
	tags: string[];
	tagsAll: boolean;
}

export const NO_CATALOG_FILTERS: CatalogFilters = {
	query: "",
	arms: "any",
	category: "any",
	sort: "new",
	eyes: false,
	tags: [],
	tagsAll: false,
};

/** Cuántas etiquetas se pueden pedir a la vez: las que acepta la API. */
export const CATALOG_TAGS_MOST = 10;

/** Una etiqueta y cuántas skins del catálogo la llevan. */
export interface CatalogTag {
	tag: string;
	count: number;
}

export interface CatalogSkin {
	id: string;
	name: string;
	/** Modelo de brazos con el que se dibujó la skin. */
	model: SkinModel;
	/** El PNG de la skin: es la dirección que se le pasa a Mojang al equiparla. */
	png: string;
	/**
	 * La skin puesta y girada, como en las tarjetas de Mosaicary (`render`, 256×256, en
	 * caché para siempre); la vista frontal plana si la respuesta no la trae.
	 */
	picture: string;
	/**
	 * La misma imagen con los ojos parpadeando y mirando a los lados (APNG), para las skins
	 * con ojos vivos de Fresh Moves; `null` en las demás.
	 */
	live: string | null;
	/** La página de la skin en Mosaicary. */
	page: string;
	/** Quién la dibujó, o quién la publicó, o el jugador que la usa. */
	credit: string | null;
	/** Si `credit` es el jugador que la lleva puesta, no su autor. */
	worn: boolean;
}

export interface CatalogPage {
	skins: CatalogSkin[];
	page: number;
	pages: number;
}

/**
 * La dirección del PNG de una skin para mostrarla, no para ponérsela: con `for=view`
 * Mosaicary no lo cuenta como descarga ni contra el límite diario de descargas por
 * dirección (50 skins distintas). La descarga es la de `uploadSkinUrl`, al guardar.
 */
export function catalogViewUrl(png: string): string {
	const url = new URL(png);
	url.searchParams.set("for", "view");
	return url.toString();
}

/** Dirección de una página del catálogo. */
export function catalogUrl(filters: CatalogFilters, page: number): string {
	const params = new URLSearchParams({
		size: "64",
		per: String(CATALOG_PAGE_SIZE),
		page: String(Math.max(1, Math.floor(page))),
	});
	const words = filters.query.trim().slice(0, 100);
	if (words) params.set("q", words);
	if (filters.arms !== "any") params.set("arms", filters.arms);
	if (filters.category !== "any") params.set("category", filters.category);
	if (filters.sort !== "new") params.set("sort", filters.sort);
	if (filters.eyes) params.set("eyes", "1");
	// Ordenadas: las mismas etiquetas elegidas en otro orden son la misma dirección.
	const tags = [...new Set(filters.tags)].sort().slice(0, CATALOG_TAGS_MOST);
	if (tags.length) {
		params.set("tag", tags.join(","));
		if (filters.tagsAll && tags.length > 1) params.set("tags", "all");
	}
	return `${MOSAICARY_SITE}/api/skins?${params}`;
}

/** Las etiquetas de las skins de 64 px, las más usadas primero. Lanza si Mosaicary no responde bien. */
export async function fetchCatalogTags(
	signal?: AbortSignal,
): Promise<CatalogTag[]> {
	const response = await fetch(`${MOSAICARY_SITE}/api/skins/tags?size=64`, {
		signal,
		headers: { accept: "application/json" },
	});
	if (!response.ok) throw new Error(`Mosaicary: HTTP ${response.status}`);
	return parseCatalogTags(await response.json());
}

/** La lista de etiquetas, validada: lo que no tenga forma de etiqueta se descarta. */
export function parseCatalogTags(data: unknown): CatalogTag[] {
	const list =
		data &&
		typeof data === "object" &&
		Array.isArray((data as { tags?: unknown }).tags)
			? (data as { tags: unknown[] }).tags
			: [];
	const tags: CatalogTag[] = [];
	for (const raw of list) {
		if (!raw || typeof raw !== "object") continue;
		const { tag, count } = raw as { tag?: unknown; count?: unknown };
		if (typeof tag !== "string" || !tag.trim() || tag.length > 60) continue;
		if (typeof count !== "number" || !(count > 0)) continue;
		tags.push({ tag: tag.trim(), count: Math.floor(count) });
	}
	return tags;
}

// Solo direcciones del propio Mosaicary: el PNG se le pasa a Mojang tal cual, y una
// respuesta alterada no debe poder colar otra cosa.
function fromMosaicary(value: unknown): value is string {
	return typeof value === "string" && value.startsWith(`${MOSAICARY_SITE}/`);
}

function nameOf(value: unknown): string | null {
	if (!value || typeof value !== "object") return null;
	const name = (value as { name?: unknown }).name;
	return typeof name === "string" && name.trim() ? name.trim() : null;
}

/**
 * Una página de la respuesta, validada. Lo que no tenga la forma esperada se descarta
 * en vez de romper la lista entera.
 */
export function parseCatalogPage(data: unknown): CatalogPage {
	const body = (data && typeof data === "object" ? data : {}) as Record<
		string,
		unknown
	>;
	const list = Array.isArray(body.skins) ? body.skins : [];
	const seen = new Set<string>();
	const skins: CatalogSkin[] = [];
	for (const raw of list) {
		if (!raw || typeof raw !== "object") continue;
		const one = raw as Record<string, unknown>;
		if (typeof one.id !== "string" || seen.has(one.id)) continue;
		const picture = fromMosaicary(one.render)
			? one.render
			: fromMosaicary(one.front)
				? one.front
				: null;
		if (!fromMosaicary(one.png) || !picture || !fromMosaicary(one.page))
			continue;
		seen.add(one.id);
		const author = nameOf(one.author);
		const wearer =
			typeof one.wearer === "string" && one.wearer.trim()
				? one.wearer.trim()
				: null;
		// Una skin encontrada en jugadores no tiene autor conocido: se nombra al jugador
		// que la usa, nunca a la cuenta de servicio que la publica.
		const found = one.found === true;
		const credit = author ?? (found ? wearer : nameOf(one.publisher));
		skins.push({
			id: one.id,
			name:
				typeof one.name === "string" && one.name.trim()
					? one.name.trim()
					: one.id,
			model: one.arms === "slim" ? "slim" : "classic",
			png: one.png,
			picture,
			live: fromMosaicary(one.live) ? one.live : null,
			page: one.page,
			credit,
			worn: !author && found && credit !== null,
		});
	}
	const page = Number(body.page) >= 1 ? Math.floor(Number(body.page)) : 1;
	const pages =
		Number(body.pages) >= page ? Math.floor(Number(body.pages)) : page;
	return { skins, page, pages };
}

/** Cuánto vale lo ya cargado de unos filtros: volver a ellos dentro de este tiempo no pide nada. */
const KEPT_MS = 5 * 60 * 1000;
/** Cuántas combinaciones de filtros se guardan como mucho. */
const KEPT_MOST = 24;

/** Lo cargado de unos filtros: todas sus páginas hasta `page`. */
export interface CatalogLoaded extends CatalogPage {
	at: number;
}

// Lo cargado por cada combinación de filtros: pasar de Alex a Steve y volver no repite la
// petición, tampoco al cerrar el catálogo y abrirlo otra vez.
const loaded = new Map<string, CatalogLoaded>();

/** Lo cargado de estos filtros, si es reciente. */
export function loadedFor(filters: CatalogFilters): CatalogPage | null {
	const kept = loaded.get(catalogUrl(filters, 1));
	return kept && Date.now() - kept.at < KEPT_MS ? kept : null;
}

/** Guarda lo cargado de estos filtros, desplazando lo más antiguo. */
export function keepLoaded(filters: CatalogFilters, page: CatalogPage) {
	const key = catalogUrl(filters, 1);
	loaded.delete(key);
	loaded.set(key, { ...page, at: Date.now() });
	while (loaded.size > KEPT_MOST) loaded.delete(loaded.keys().next().value!);
}

/** Una página del catálogo. Lanza si Mosaicary no responde bien. */
export async function fetchCatalogPage(
	filters: CatalogFilters,
	page: number,
	signal?: AbortSignal,
): Promise<CatalogPage> {
	const response = await fetch(catalogUrl(filters, page), {
		signal,
		headers: { accept: "application/json" },
	});
	if (!response.ok) throw new Error(`Mosaicary: HTTP ${response.status}`);
	return parseCatalogPage(await response.json());
}
