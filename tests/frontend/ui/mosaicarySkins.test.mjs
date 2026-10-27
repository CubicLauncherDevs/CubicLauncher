import { expect, test } from "bun:test";
import {
	CATALOG_PAGE_SIZE,
	NO_CATALOG_FILTERS,
	catalogUrl,
	keepLoaded,
	loadedFor,
	parseCatalogPage,
	parseCatalogTags,
} from "../../../src/lib/api/mosaicarySkins.ts";

const SITE = "https://mosaicary.com";

// Una skin como la devuelve /api/skins.
function entry(over = {}) {
	return {
		id: "TTQ9RtVv7jt2",
		name: "Stick figure skin",
		arms: "slim",
		width: 64,
		height: 64,
		png: `${SITE}/api/skins/TTQ9RtVv7jt2.png`,
		front: `${SITE}/api/skins/TTQ9RtVv7jt2/front.png?v=0`,
		render: `${SITE}/api/skins/TTQ9RtVv7jt2/render.png?v=0.1`,
		page: `${SITE}/models/TTQ9RtVv7jt2`,
		author: null,
		publisher: { name: "Mosaicary Skins", page: `${SITE}/u/MosaicarySkn` },
		found: true,
		wearer: "Kunumii",
		...over,
	};
}

test("pide solo skins que Mojang acepta, con búsqueda y filtros", () => {
	const url = new URL(
		catalogUrl(
			{
				query: "  knight  ",
				arms: "slim",
				category: "creatures",
				sort: "popular",
				eyes: true,
			},
			3,
		),
	);
	expect(url.origin + url.pathname).toBe(`${SITE}/api/skins`);
	expect(url.searchParams.get("size")).toBe("64");
	expect(url.searchParams.get("per")).toBe(String(CATALOG_PAGE_SIZE));
	expect(url.searchParams.get("page")).toBe("3");
	expect(url.searchParams.get("q")).toBe("knight");
	expect(url.searchParams.get("arms")).toBe("slim");
	expect(url.searchParams.get("category")).toBe("creatures");
	expect(url.searchParams.get("sort")).toBe("popular");
	expect(url.searchParams.get("eyes")).toBe("1");
});

test("sin filtros solo manda tamaño, página y cuántas", () => {
	const url = new URL(catalogUrl({ ...NO_CATALOG_FILTERS, query: "   " }, 0));
	expect([...url.searchParams.keys()].sort()).toEqual([
		"page",
		"per",
		"size",
	]);
	expect(url.searchParams.get("page")).toBe("1");
});

test("lee una página: modelo, enlaces y a quién nombrar", () => {
	const page = parseCatalogPage({
		skins: [entry()],
		page: 1,
		per: 24,
		total: 472,
		pages: 20,
	});
	expect(page.page).toBe(1);
	expect(page.pages).toBe(20);
	expect(page.skins).toHaveLength(1);
	const [skin] = page.skins;
	expect(skin.model).toBe("slim");
	expect(skin.png).toBe(`${SITE}/api/skins/TTQ9RtVv7jt2.png`);
	// La skin puesta y girada, como en las tarjetas de Mosaicary.
	expect(skin.picture).toBe(
		`${SITE}/api/skins/TTQ9RtVv7jt2/render.png?v=0.1`,
	);
	// Encontrada en jugadores: se nombra a quien la usa, no a la cuenta que la publica.
	expect(skin.credit).toBe("Kunumii");
	expect(skin.worn).toBe(true);
});

test("nombra al autor si lo hay, y si no a quien la publicó", () => {
	const page = parseCatalogPage({
		skins: [
			entry({
				id: "a",
				author: { name: "Someone", url: "https://example.com" },
			}),
			entry({
				id: "b",
				found: false,
				wearer: null,
				publisher: { name: "skype", page: `${SITE}/u/x` },
			}),
		],
	});
	expect(page.skins.map((one) => [one.credit, one.worn])).toEqual([
		["Someone", false],
		["skype", false],
	]);
});

test("las etiquetas van ordenadas, y «todas» solo con más de una", () => {
	const both = new URL(
		catalogUrl(
			{
				...NO_CATALOG_FILTERS,
				tags: ["red", "cute", "red"],
				tagsAll: true,
			},
			1,
		),
	);
	expect(both.searchParams.get("tag")).toBe("cute,red");
	expect(both.searchParams.get("tags")).toBe("all");
	const one = new URL(
		catalogUrl({ ...NO_CATALOG_FILTERS, tags: ["red"], tagsAll: true }, 1),
	);
	expect(one.searchParams.get("tag")).toBe("red");
	expect(one.searchParams.has("tags")).toBe(false);
});

test("la lista de etiquetas descarta lo que no lo es", () => {
	expect(
		parseCatalogTags({
			tags: [
				{ tag: "black", count: 543 },
				{ tag: "  ", count: 3 },
				{ tag: "blue", count: 0 },
				{ tag: 7, count: 2 },
				null,
				{ tag: "cute", count: 288.6 },
			],
		}),
	).toEqual([
		{ tag: "black", count: 543 },
		{ tag: "cute", count: 288 },
	]);
	expect(parseCatalogTags("nada")).toEqual([]);
});

test("lo cargado de unos filtros se recupera sin pedirlo otra vez", () => {
	const slim = { ...NO_CATALOG_FILTERS, arms: "slim" };
	const loadedPage = { skins: [], page: 2, pages: 9 };
	keepLoaded(slim, loadedPage);
	expect(loadedFor({ ...slim })).toMatchObject(loadedPage);
	expect(loadedFor({ ...slim, arms: "classic" })).toBeNull();
	// El texto buscado cuenta como lo manda la dirección: sin espacios sobrantes.
	expect(loadedFor({ ...slim, query: "  " })).toMatchObject(loadedPage);
});

test("los ojos vivos solo si la dirección es de Mosaicary", () => {
	const live = `${SITE}/api/skins/TTQ9RtVv7jt2/live.png?v=0.1`;
	const page = parseCatalogPage({
		skins: [
			entry({ id: "a", live }),
			entry({ id: "b", live: "https://example.com/a.png" }),
			entry({ id: "c" }),
		],
	});
	expect(page.skins.map((one) => one.live)).toEqual([live, null, null]);
});

test("sin render usa la vista frontal plana", () => {
	const [skin] = parseCatalogPage({
		skins: [entry({ render: undefined })],
	}).skins;
	expect(skin.picture).toBe(`${SITE}/api/skins/TTQ9RtVv7jt2/front.png?v=0`);
});

test("descarta lo que no sea de Mosaicary: esa dirección se le pasa a Mojang", () => {
	const page = parseCatalogPage({
		skins: [
			entry({ id: "x", png: "https://example.com/evil.png" }),
			entry({
				id: "y",
				render: "https://example.com/y.png",
				front: "http://mosaicary.com/api/skins/y/front.png",
			}),
			entry({ id: "z", page: 42 }),
			null,
			"text",
			entry({ id: "ok" }),
			entry({ id: "ok" }),
		],
	});
	expect(page.skins.map((one) => one.id)).toEqual(["ok"]);
});

test("una respuesta rota da una página vacía, no una excepción", () => {
	expect(parseCatalogPage(null)).toEqual({ skins: [], page: 1, pages: 1 });
	expect(parseCatalogPage({ skins: "no", page: -2, pages: "x" })).toEqual({
		skins: [],
		page: 1,
		pages: 1,
	});
});

test("un nombre vacío deja el id y unos brazos raros quedan en clásico", () => {
	const [skin] = parseCatalogPage({
		skins: [entry({ name: "   ", arms: "weird" })],
	}).skins;
	expect(skin.name).toBe("TTQ9RtVv7jt2");
	expect(skin.model).toBe("classic");
});
