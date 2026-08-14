import { marked, type Renderer, type Token } from "marked";
import DOMPurify, { type Config } from "dompurify";

const sanitizer = DOMPurify;

const DEFAULT_MAX_SOURCE_LENGTH = 120_000;
const CACHE_MAX_SIZE = 50;

interface RenderOptions {
	baseUrl?: string;
	maxLength?: number;
}

const cache = new Map<string, string>();

const SANITIZE_CONFIG: Config = {
	USE_PROFILES: { html: true },
	ALLOWED_TAGS: [
		"p",
		"br",
		"hr",
		"h1",
		"h2",
		"h3",
		"h4",
		"h5",
		"h6",
		"strong",
		"em",
		"u",
		"s",
		"a",
		"img",
		"ul",
		"ol",
		"li",
		"blockquote",
		"code",
		"pre",
		"table",
		"thead",
		"tbody",
		"tr",
		"th",
		"td",
		"div",
		"span",
	],
	ALLOWED_ATTR: [
		"href",
		"src",
		"alt",
		"title",
		"class",
		"target",
		"rel",
		"loading",
		"decoding",
	],
	FORCE_BODY: true,
	ALLOW_DATA_ATTR: false,
};

function escapeAttr(value: string): string {
	return value
		.replace(/&/g, "&amp;")
		.replace(/"/g, "&quot;")
		.replace(/</g, "&lt;")
		.replace(/>/g, "&gt;");
}

function isAbsoluteUrl(url: string): boolean {
	const trimmed = url.trim();
	if (trimmed === "") return true;
	return (
		/^https?:/i.test(trimmed) ||
		/^mailto:/i.test(trimmed) ||
		/^tel:/i.test(trimmed) ||
		trimmed.startsWith("//")
	);
}

function resolveUrl(url: string, baseUrl?: string): string {
	if (!baseUrl) return url;
	const trimmed = url.trim();
	if (trimmed === "" || isAbsoluteUrl(trimmed) || trimmed.startsWith("#")) {
		return trimmed;
	}
	try {
		return new URL(trimmed, baseUrl).href;
	} catch {
		return trimmed;
	}
}

function truncateSource(source: string, maxLength: number): string {
	if (source.length <= maxLength) return source;

	const cutoff = source.lastIndexOf("\n\n", maxLength);
	const endIndex = cutoff > maxLength * 0.8 ? cutoff : maxLength;

	return (
		source.slice(0, endIndex) +
		"\n\n---\n\n*README truncated because it is too long.*"
	);
}

function getCacheKey(source: string, options: RenderOptions): string {
	return `${options.baseUrl ?? ""}::${options.maxLength ?? 0}::${source.length}::${source}`;
}

function getCached(key: string): string | undefined {
	const html = cache.get(key);
	if (html === undefined) return undefined;
	cache.delete(key);
	cache.set(key, html);
	return html;
}

function setCache(key: string, html: string): void {
	if (cache.size >= CACHE_MAX_SIZE) {
		const firstKey = cache.keys().next().value as string | undefined;
		if (firstKey) cache.delete(firstKey);
	}
	cache.set(key, html);
}

function createRenderer(baseUrl?: string): Renderer {
	const renderer = new marked.Renderer();

	renderer.link = function ({ href, title, tokens }) {
		const text = (
			this as unknown as {
				parser: { parseInline(tokens: Token[]): string };
			}
		).parser.parseInline(tokens);

		const resolvedHref = resolveUrl(href, baseUrl);
		const isAnchor = resolvedHref.startsWith("#");

		let html = `<a href="${escapeAttr(resolvedHref)}"`;
		if (!isAnchor) {
			html += ' target="_blank" rel="noopener noreferrer"';
		}
		if (title) {
			html += ` title="${escapeAttr(title)}"`;
		}
		return `${html}>${text}</a>`;
	};

	renderer.image = function ({ href, title, text }) {
		const resolvedSrc = resolveUrl(href, baseUrl);
		let html = `<img src="${escapeAttr(resolvedSrc)}" alt="${escapeAttr(text)}" loading="lazy" decoding="async"`;
		if (title) {
			html += ` title="${escapeAttr(title)}"`;
		}
		return `${html}>`;
	};

	return renderer;
}

sanitizer.addHook("afterSanitizeAttributes", (node) => {
	if (!(node instanceof HTMLElement)) return;

	if (node.tagName === "A") {
		const href = node.getAttribute("href") ?? "";
		if (href.startsWith("#")) {
			node.removeAttribute("target");
			node.removeAttribute("rel");
		} else {
			node.setAttribute("target", "_blank");
			node.setAttribute("rel", "noopener noreferrer");
		}
	}

	if (node.tagName === "IMG") {
		node.setAttribute("loading", "lazy");
		node.setAttribute("decoding", "async");
	}
});

export function renderMarkdown(
	source: string | null | undefined,
	options: RenderOptions = {},
): string {
	if (!source) return "";

	const maxLength = options.maxLength ?? DEFAULT_MAX_SOURCE_LENGTH;
	const cacheKey = getCacheKey(source, options);

	const cached = getCached(cacheKey);
	if (cached !== undefined) return cached;

	const truncated = truncateSource(source, maxLength);

	try {
		const html = marked.parse(truncated, {
			gfm: true,
			breaks: false,
			renderer: createRenderer(options.baseUrl),
		}) as string;

		const safe = sanitizer.sanitize(html, SANITIZE_CONFIG) as string;

		setCache(cacheKey, safe);
		return safe;
	} catch (e) {
		console.error("[renderMarkdown] failed to render markdown:", e);
		return "";
	}
}
