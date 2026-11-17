import { describe, expect, it } from "vitest";
import { isPluginTab, panelTabId, parsePanelTab } from "./panels";

describe("플러그인 패널 탭 id", () => {
	it("왕복", () => {
		const id = panelTabId("word-count", "main");
		expect(id).toBe("plugin:word-count:main");
		expect(isPluginTab(id)).toBe(true);
		expect(parsePanelTab(id)).toEqual({ pluginId: "word-count", panelId: "main" });
	});
	it("화면 탭은 아님", () => {
		expect(isPluginTab("context")).toBe(false);
		expect(parsePanelTab("context")).toBeNull();
		expect(parsePanelTab("plugin:broken")).toBeNull();
	});
});
