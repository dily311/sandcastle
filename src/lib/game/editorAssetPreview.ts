import { convertFileSrc } from "@tauri-apps/api/core";

/** Resolve a game-relative image path using Tauri's already-enabled asset protocol. */
export function editorAssetPreviewUrl(assetRoot: string, filePath: string | null | undefined): string {
	if (!filePath?.trim()) return "";
	if (/^(https?:|data:|blob:|asset:)/i.test(filePath)) return filePath;
	if (!assetRoot) return "";
	if (/^(?:[a-z]:[\\/]|\\\\|\/)/i.test(filePath)) return convertFileSrc(filePath);

	const separator = assetRoot.includes("\\") ? "\\" : "/";
	const relativePath = filePath.replace(/[\\/]+/g, separator).replace(/^[\\/]+/, "");
	return convertFileSrc(`${assetRoot}${separator}${relativePath}`);
}
