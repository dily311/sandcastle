// 프리셋 태그를 단 그림의 생성 정보(meta) → 이미지 생성 설정에 넣을 값.
// 갤러리에 올린 NovelAI·WebUI 그림, 앱이 만든 그림 모두 같은 이름(네거티브·단계·…)으로 정리돼 있다 (service/image_meta.rs)
import type { MetaItem } from "$ipc/bindings";

export interface PresetPatch {
	negativePrompt?: string;
	steps?: number;
	scale?: number;
	sampler?: string;
	size?: string;
	model?: string;
}

/** NovelAI 이름으로 맞추기 (WebUI 샘플러 이름은 그대로 두면 NovelAI가 거절하므로 아는 것만) */
const SAMPLER: Record<string, string> = {
	"euler a": "k_euler_ancestral",
	euler: "k_euler",
	"dpm++ 2s a": "k_dpmpp_2s_ancestral",
	"dpm++ 2m sde": "k_dpmpp_2m_sde",
	"dpm++ 2m": "k_dpmpp_2m",
	"dpm++ sde": "k_dpmpp_sde",
};

export function presetFromMeta(meta: MetaItem[]): PresetPatch {
	const get = (k: string) => meta.find((m) => m.key === k)?.value.trim() ?? "";
	const out: PresetPatch = {};
	const neg = get("네거티브");
	if (neg) out.negativePrompt = neg;
	const steps = Number.parseInt(get("단계"), 10);
	if (Number.isFinite(steps) && steps > 0) out.steps = Math.min(50, steps);
	const scale = Number.parseFloat(get("프롬프트 반영"));
	if (Number.isFinite(scale) && scale > 0) out.scale = Math.min(10, scale);
	const sampler = get("샘플러");
	if (sampler) out.sampler = sampler.startsWith("k_") ? sampler : (SAMPLER[sampler.toLowerCase()] ?? sampler);
	const size = /^(\d{2,4})\s*x\s*(\d{2,4})$/i.exec(get("크기"));
	if (size) out.size = `${size[1]}x${size[2]}`;
	const model = get("모델");
	if (model.startsWith("nai-diffusion")) out.model = model;
	return out;
}

/** 화면에 보일 한 줄 요약 */
export function presetSummary(p: PresetPatch): string {
	return [p.size, p.steps && `${p.steps}단계`, p.scale && `반영 ${p.scale}`, p.sampler].filter(Boolean).join(" · ");
}
