import { describe, expect, it } from "vitest";
import { presetFromMeta, presetSummary } from "./presetFromImage";

const m = (key: string, value: string) => ({ key, value });

describe("그림 생성 정보 → 프리셋", () => {
	it("NovelAI 정보를 그대로 쓴다", () => {
		const p = presetFromMeta([m("네거티브", "lowres"), m("단계", "28"), m("프롬프트 반영", "5"), m("샘플러", "k_euler_ancestral"), m("크기", "832x1216"), m("모델", "nai-diffusion-4-5-full")]);
		expect(p).toEqual({ negativePrompt: "lowres", steps: 28, scale: 5, sampler: "k_euler_ancestral", size: "832x1216", model: "nai-diffusion-4-5-full" });
		expect(presetSummary(p)).toBe("832x1216 · 28단계 · 반영 5 · k_euler_ancestral");
	});
	it("WebUI 이름은 NovelAI 이름으로, 범위 밖은 자르고, 모르는 모델은 넣지 않는다", () => {
		const p = presetFromMeta([m("샘플러", "Euler a"), m("단계", "80"), m("프롬프트 반영", "12"), m("모델", "anything-v5")]);
		expect(p).toEqual({ sampler: "k_euler_ancestral", steps: 50, scale: 10 });
	});
	it("정보가 없으면 빈 묶음", () => {
		expect(presetFromMeta([m("원래 파일", "a.png")])).toEqual({});
	});
});
