// 플러그인 패널 탭 id (6-D-6): "plugin:<플러그인 id>:<패널 id>"
export const PANEL_PREFIX = "plugin:";

export function panelTabId(pluginId: string, panelId: string) {
	return `${PANEL_PREFIX}${pluginId}:${panelId}`;
}

export function isPluginTab(tab: string) {
	return tab.startsWith(PANEL_PREFIX);
}

export function parsePanelTab(tab: string): { pluginId: string; panelId: string } | null {
	if (!isPluginTab(tab)) return null;
	const rest = tab.slice(PANEL_PREFIX.length);
	const i = rest.indexOf(":");
	if (i <= 0) return null;
	return { pluginId: rest.slice(0, i), panelId: rest.slice(i + 1) };
}
