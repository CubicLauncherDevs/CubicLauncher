import type { ModDto } from "$lib/types/types";
import { localModToMarket, type MarketProject } from "$lib/types/market";

const searchText = new WeakMap<MarketProject, string>();

function sameMod(a: ModDto, b: ModDto): boolean {
	return (
		a.filename === b.filename &&
		a.name === b.name &&
		a.version === b.version &&
		a.description === b.description &&
		a.icon === b.icon &&
		a.enabled === b.enabled &&
		a.sha1 === b.sha1 &&
		a.file_size === b.file_size &&
		a.source === b.source &&
		a.project_id === b.project_id &&
		a.slug === b.slug &&
		a.icon_revision === b.icon_revision &&
		(a.authors === b.authors ||
			(!!a.authors &&
				!!b.authors &&
				a.authors.length === b.authors.length &&
				a.authors.every((author, i) => author === b.authors![i])))
	);
}

/** Preserve project/array identity so a rescan only updates changed rows. */
export function reconcileLocalProjects(
	previous: MarketProject[],
	mods: ModDto[],
): MarketProject[] {
	const byFile = new Map(
		previous.map((item) => [item.installed!.filename, item]),
	);
	const next = mods.map((mod) => {
		const old = byFile.get(mod.filename);
		return old?.installed && sameMod(old.installed, mod)
			? old
			: localModToMarket(mod);
	});
	return sameProjectList(previous, next) ? previous : next;
}

export function sameProjectList(
	a: MarketProject[],
	b: MarketProject[],
): boolean {
	return a.length === b.length && a.every((item, index) => item === b[index]);
}

export function matchesLocalQuery(
	project: MarketProject,
	query: string,
): boolean {
	if (!query) return true;
	let text = searchText.get(project);
	if (text === undefined) {
		text = [
			project.title,
			project.description,
			project.author,
			project.installed?.filename ?? "",
		]
			.join("\n")
			.toLowerCase();
		searchText.set(project, text);
	}
	return text.includes(query);
}
