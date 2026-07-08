import { marked } from "marked";
import DOMPurify from "dompurify";

const sanitizer = DOMPurify;

export function renderMarkdown(source: string | null | undefined): string {
	if (!source) return "";

	const html = marked.parse(source, {
		async: false,
		gfm: true,
		breaks: false,
	}) as string;

	const safe = sanitizer.sanitize(html, {
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
			"style",
			"target",
			"rel",
		],
	}) as string;

	return safe.replaceAll("<img ", '<img loading="lazy" ');
}
