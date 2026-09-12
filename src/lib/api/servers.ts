import { Channel, invoke } from "@tauri-apps/api/core";

export type ResourcePolicy = "prompt" | "enabled" | "disabled";

export interface ServerInput {
	name: string;
	address: string;
	resourcePolicy: ResourcePolicy;
}

export interface ServerDto extends ServerInput {
	index: number;
	icon: string | null;
}

export interface ServerList {
	revision: string;
	servers: ServerDto[];
}

export interface ServerStatus {
	online: boolean;
	players: number | null;
	maxPlayers: number | null;
	ping: number | null;
	motd: string;
	version: string | null;
	icon: string | null;
}

export interface ServerStatusEvent {
	index: number | null;
	status: ServerStatus | null;
	done: boolean;
}

export type ServerAction =
	| { type: "add"; server: ServerInput }
	| { type: "edit"; index: number; server: ServerInput }
	| { type: "delete"; index: number }
	| { type: "move"; index: number; direction: "up" | "down" };

export function getInstanceServers(instanceId: string): Promise<ServerList> {
	return invoke("get_instance_servers", { instanceId });
}

export function serverAction(
	instanceId: string,
	revision: string,
	action: ServerAction,
): Promise<ServerList> {
	return invoke("instance_server_action", { instanceId, revision, action });
}

/** Returns cancellation immediately, even if the native start command is pending. */
export function pingInstanceServers(
	instanceId: string,
	revision: string,
	onStatus: (event: ServerStatusEvent) => void,
	onError: (error: unknown) => void,
): () => void {
	let cancelled = false;
	let requestId: string | null = null;
	const channel = new Channel<ServerStatusEvent>();
	channel.onmessage = (event) => {
		if (!cancelled) onStatus(event);
	};
	const cancelNative = (id: string) => {
		void invoke("cancel_instance_server_ping", { requestId: id }).catch(
			() => {},
		);
	};
	void invoke<string>("start_instance_server_ping", {
		instanceId,
		revision,
		onStatus: channel,
	})
		.then((id) => {
			requestId = id;
			if (cancelled) cancelNative(id);
		})
		.catch((error) => {
			if (!cancelled) onError(error);
		});
	return () => {
		if (cancelled) return;
		cancelled = true;
		if (requestId) cancelNative(requestId);
	};
}
