import { describe, expect, it } from "vitest";
import { apiLists, embeddingModelInfo, embeddingModelsFor, firstModel, modelIdsOf, modelLabel } from "./providers";

// 앱에 내장한 모델 목록(src-tauri/resources/info/*.json)이 화면이 기대하는 모양인지 — 사람이 JSON을 고쳐도 깨지지 않게
describe("내장 모델 목록", () => {
	const chat = apiLists.filter((p) => !["elevenlabs", "novelai", "openrouter"].includes(p.provider));

	it("채팅 공급자마다 모델이 있고 id가 겹치지 않는다", () => {
		for (const p of chat) {
			const ids = modelIdsOf(p);
			expect(ids.length, p.label).toBeGreaterThan(0);
			expect(new Set(ids).size, `${p.label}의 id 중복`).toBe(ids.length);
			expect(firstModel(p), p.label).toBe(ids[0]);
		}
	});

	it("이름과 id가 비어 있지 않고, 이름으로 되찾을 수 있다", () => {
		const openai = apiLists.find((p) => p.provider === "OpenAI");
		const first = openai?.modelList[0]?.model[0];
		expect(first?.label && first.modelId).toBeTruthy();
		expect(modelLabel(openai, first!.modelId)).toBe(first!.label);
		// 목록에 없는 id는 그대로
		expect(modelLabel(openai, "my-own-model")).toBe("my-own-model");
	});

	it("Google 연결의 형식은 백엔드가 아는 gemini", () => {
		expect(apiLists.find((p) => p.provider === "google")?.formatType).toBe("gemini");
	});

	it("음성·그림 연결의 모델 목록도 id를 가진다", () => {
		expect(modelIdsOf(apiLists.find((p) => p.provider === "elevenlabs")).length).toBeGreaterThan(0);
		expect(modelIdsOf(apiLists.find((p) => p.provider === "novelai")).length).toBeGreaterThan(0);
	});
});

describe("내장 임베딩 모델 목록", () => {
	it("공급자에 맞는 목록, 없으면 전부", () => {
		const openai = embeddingModelsFor("OpenAI");
		expect(openai.length).toBeGreaterThan(0);
		expect(openai.every((m) => m.provider === "openai")).toBe(true);
		expect(embeddingModelsFor("openrouter").length).toBeGreaterThan(openai.length);
	});

	it("기본 차원·최대 입력을 읽는다", () => {
		const m = embeddingModelInfo("text-embedding-3-small");
		expect(m?.dimensions).toBe(1536);
		expect(m?.adjustable).toBe(true);
		expect(m?.maxInputTokens).toBe(8192);
		expect(m?.price).toBe(0.02);
		expect(embeddingModelInfo("없는-모델")).toBeUndefined();
	});

	it("모든 모델에 id·이름이 있고 id가 겹치지 않는다", () => {
		const all = embeddingModelsFor(undefined);
		expect(all.every((m) => m.modelId && m.label)).toBe(true);
		expect(new Set(all.map((m) => `${m.provider}/${m.modelId}`)).size).toBe(all.length);
	});
});
