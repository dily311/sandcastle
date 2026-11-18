import { describe, expect, it } from "vitest";
import { gather, PER_PLUGIN_MS } from "./promptHook";

const req = { requestId: "r1", conversationId: "v1", characterId: "c1", userInput: "hi", plugins: ["a", "b", "gone", "slow"] };

describe("프롬프트 훅 모으기", () => {
	it("백엔드가 고른 플러그인 중 도는 것만, 블록 모양을 맞춰서", async () => {
		const called: [string, number][] = [];
		const emit = async (id: string, _e: string, data: unknown, ms: number) => {
			called.push([id, ms]);
			expect(data).toEqual({ conversationId: "v1", characterId: "c1", userInput: "hi" });
			if (id === "a") return { blocks: [{ text: "rule", position: "rules" }, { text: "deep", position: "history", depth: 2.7 }, { text: "  " }] };
			if (id === "b") return ["plain string block"];
			throw new Error("응답 없음");
		};
		const r = await gather(req, ["a", "b", "slow"], emit);
		expect(called).toEqual([
			["a", PER_PLUGIN_MS],
			["b", PER_PLUGIN_MS],
			["slow", PER_PLUGIN_MS],
		]);
		expect(r.answers).toEqual([
			{
				pluginId: "a",
				blocks: [
					{ text: "rule", position: "rules", depth: null },
					{ text: "deep", position: "history", depth: 2 },
				],
			},
			{ pluginId: "b", blocks: [{ text: "plain string block", position: "rules", depth: null }] },
		]);
		expect(r.errors).toEqual([{ pluginId: "slow", message: "응답 없음" }]);
	});
});
