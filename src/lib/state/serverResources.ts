import {
	getInstanceServerIcons,
	pingInstanceServers,
	type ServerDto,
	type ServerList,
	type ServerAction,
	type ServerStatus,
} from "../api/servers";

export interface ServerRow extends ServerDto {
	key: number;
}
export interface ServerResourceView {
	statuses: Record<number, ServerStatus>;
	icons: Record<number, string | null>;
	querying: boolean;
}

// Budgets account for UTF-16 text and entry overhead, not the browser's decoded
// image surfaces. Only the current page/selection is additionally held for display.
const MAX_ENTRIES = 100;
const MAX_BYTES = 512 * 1024;
const STATUS_TTL = 30_000;
const ICON_TTL = 5 * 60_000;

export class ResourceCache<K, V> {
	private entries = new Map<
		K,
		{ value: V; bytes: number; expires: number }
	>();
	bytes = 0;
	constructor(
		private now: () => number,
		private maxEntries = MAX_ENTRIES,
		private maxBytes = MAX_BYTES,
	) {}
	get size() {
		return this.entries.size;
	}
	get(key: K): V | undefined {
		const entry = this.entries.get(key);
		if (!entry) return undefined;
		if (entry.expires <= this.now()) {
			this.delete(key);
			return undefined;
		}
		this.entries.delete(key);
		this.entries.set(key, entry);
		return entry.value;
	}
	set(key: K, value: V, bytes: number, ttl: number) {
		this.delete(key);
		this.prune();
		if (bytes > this.maxBytes) return;
		while (
			this.entries.size >= this.maxEntries ||
			this.bytes + bytes > this.maxBytes
		) {
			const oldest = this.entries.keys().next();
			if (oldest.done) break;
			this.delete(oldest.value);
		}
		this.entries.set(key, { value, bytes, expires: this.now() + ttl });
		this.bytes += bytes;
	}
	delete(key: K) {
		const entry = this.entries.get(key);
		if (entry) {
			this.bytes -= entry.bytes;
			this.entries.delete(key);
		}
	}
	prune() {
		const now = this.now();
		for (const [key, entry] of this.entries)
			if (entry.expires <= now) this.delete(key);
	}
	clear() {
		this.entries.clear();
		this.bytes = 0;
	}
}

function addressKey(address: string): string {
	let key = address.trim().toLowerCase();
	if (
		key.endsWith(":25565") &&
		(key.startsWith("[") || key.indexOf(":") === key.lastIndexOf(":"))
	)
		key = key.slice(0, -6);
	if (key.startsWith("[") && key.endsWith("]")) key = key.slice(1, -1);
	return key;
}

type Transport = {
	icons: typeof getInstanceServerIcons;
	ping: typeof pingInstanceServers;
};

/** Small, non-reactive controller. Publish immutable snapshots of at most 51 rows. */
export class ServerResources {
	rows: ServerRow[] = [];
	private revision = "";
	private nextKey = 0;
	private generation = 0;
	private cancelPing: (() => void) | null = null;
	private signature = "";
	private active = new Map<number, ServerRow>();
	private statuses: Record<number, ServerStatus> = {};
	private icons: Record<number, string | null> = {};
	private querying = false;
	private disposed = false;
	private iconRead: Promise<void> | null = null;
	private statusCache: ResourceCache<string, ServerStatus>;
	private iconCache: ResourceCache<number, string | null>;
	constructor(
		private instanceId: string,
		private changed: (view: ServerResourceView) => void,
		private onError: (error: unknown) => void,
		private transport: Transport = {
			icons: getInstanceServerIcons,
			ping: pingInstanceServers,
		},
		now: () => number = Date.now,
	) {
		this.statusCache = new ResourceCache(now);
		this.iconCache = new ResourceCache(now);
	}

	setList(list: ServerList, action?: ServerAction): ServerRow[] {
		this.pause();
		const previous: (ServerRow | undefined)[] = [...this.rows];
		if (action?.type === "move") {
			const other = action.index + (action.direction === "up" ? -1 : 1);
			[previous[action.index], previous[other]] = [
				previous[other],
				previous[action.index],
			];
		} else if (action?.type === "delete") {
			const removed = previous.splice(action.index, 1)[0];
			if (removed) this.iconCache.delete(removed.key);
		}
		if (!action && list.revision !== this.revision) this.iconCache.clear();
		this.rows = list.servers.map((server, index) => {
			const old = previous[index];
			const reuse =
				old &&
				(action ||
					(old.name === server.name &&
						old.address === server.address));
			const key = reuse ? old.key : ++this.nextKey;
			if (old && (!server.hasIcon || old.address !== server.address))
				this.iconCache.delete(old.key);
			return { ...server, key };
		});
		this.revision = list.revision;
		if (!this.rows.length) {
			this.statusCache.clear();
			this.iconCache.clear();
		}
		return this.rows;
	}

	refresh() {
		this.statusCache.clear();
		this.iconCache.clear();
		this.pause();
	}

	show(rows: ServerRow[]) {
		if (this.disposed) return;
		// Defend the resource bound even if a caller forgets to paginate.
		rows = rows.slice(0, 51);
		const signature = `${this.revision}:${rows.map((row) => row.key).join(",")}`;
		if (signature === this.signature) return;
		this.pause();
		this.signature = signature;
		this.statusCache.prune();
		this.iconCache.prune();
		this.active = new Map(rows.map((row) => [row.index, row]));
		const token = this.generation;
		const targets = [];
		const missingIcons: number[] = [];
		for (const row of rows) {
			const status = this.statusCache.get(addressKey(row.address));
			if (status) this.statuses[row.index] = status;
			else targets.push({ index: row.index, address: row.address });
			if (!row.hasIcon || status?.icon) continue;
			const icon = this.iconCache.get(row.key);
			if (icon !== undefined) this.icons[row.index] = icon;
			else missingIcons.push(row.index);
		}
		this.querying = targets.length > 0;
		this.publish();
		if (targets.length) {
			this.cancelPing = this.transport.ping(
				this.instanceId,
				targets,
				(event) => {
					if (this.disposed || token !== this.generation) return;
					if (event.status)
						for (const index of event.indices) {
							const row = this.active.get(index);
							if (!row) continue;
							const status = event.status;
							this.statuses[index] = status;
							const key = addressKey(row.address);
							this.statusCache.set(
								key,
								status,
								128 +
									2 *
										(key.length +
											status.motd.length +
											(status.version?.length ?? 0) +
											(status.icon?.length ?? 0)),
								STATUS_TTL,
							);
							if (status.icon) {
								this.iconCache.delete(row.key);
								delete this.icons[index];
							}
						}
					if (event.done) this.querying = false;
					this.publish();
				},
				(error) => {
					if (this.disposed || token !== this.generation) return;
					this.querying = false;
					this.publish();
					this.onError(error);
				},
			);
		}
		if (missingIcons.length) {
			const revision = this.revision;
			const read = async () => {
				// A previous page may still hold the native file lock. Serialize
				// reads and skip intermediate pages before doing any disk work.
				if (this.disposed || token !== this.generation) return;
				try {
					const icons = await this.transport.icons(
						this.instanceId,
						revision,
						missingIcons,
					);
					if (this.disposed || token !== this.generation) return;
					for (const { index, icon } of icons) {
						const row = this.active.get(index);
						if (!row || this.statuses[index]?.icon) continue;
						this.icons[index] = icon;
						this.iconCache.set(
							row.key,
							icon,
							64 + 2 * (icon?.length ?? 0),
							ICON_TTL,
						);
					}
					this.publish();
				} catch (error) {
					if (!this.disposed && token === this.generation)
						this.onError(error);
				}
			};
			const pending = this.iconRead ? this.iconRead.then(read) : read();
			this.iconRead = pending;
			void pending.then(() => {
				if (this.iconRead === pending) this.iconRead = null;
			});
		}
	}

	waitForIcons(): Promise<void> {
		return this.iconRead ?? Promise.resolve();
	}

	private publish() {
		if (!this.disposed)
			this.changed({
				statuses: { ...this.statuses },
				icons: { ...this.icons },
				querying: this.querying,
			});
	}
	pause() {
		this.generation++;
		this.cancelPing?.();
		this.cancelPing = null;
		this.signature = "";
		this.active.clear();
		this.statuses = {};
		this.icons = {};
		this.querying = false;
		this.publish();
	}
	dispose() {
		this.pause();
		this.disposed = true;
		this.statusCache.clear();
		this.iconCache.clear();
		this.rows = [];
	}
}
