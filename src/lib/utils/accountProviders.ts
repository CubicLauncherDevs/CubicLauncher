import type { MinecraftUser } from "$lib/types/types";

export function isElyByAccount(
	user: Pick<MinecraftUser, "user_type" | "yggdrasil_server_url">,
): boolean {
	if (user.user_type !== "Yggdrasil" || !user.yggdrasil_server_url) {
		return false;
	}
	try {
		const server = user.yggdrasil_server_url.trim();
		const url = new URL(
			server.includes("://") ? server : `https://${server}`,
		);
		return (
			["http:", "https:"].includes(url.protocol) &&
			!url.port &&
			[
				"ely.by",
				"www.ely.by",
				"authserver.ely.by",
				"account.ely.by",
			].includes(url.hostname)
		);
	} catch {
		return false;
	}
}
