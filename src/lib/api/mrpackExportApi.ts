import { invoke } from "@tauri-apps/api/core";

export interface MrpackExportEntry {
	path: string;
	isDir: boolean;
	size: number;
	defaultSelected: boolean;
}

export interface MrpackExportPreview {
	author?: string;
	name: string;
	versionId: string;
	summary: string;
	dependencies: Record<string, string>;
	entries: MrpackExportEntry[];
}

export interface MrpackExportRequest {
	author?: string;
	name: string;
	versionId: string;
	summary: string;
	/** Exact file paths returned by the preview, not directory paths. */
	selectedPaths: string[];
}

export const previewMrpackExport = (id: string) =>
	invoke<MrpackExportPreview>("preview_mrpack_export", { id });

export const exportInstanceMrpack = (
	id: string,
	dest: string,
	request: MrpackExportRequest,
) => invoke<string>("export_instance_mrpack", { id, dest, request });
