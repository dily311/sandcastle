// AI 공급자 목록 (설정 > API 키의 "공급자" 고르기, 모델 이름 추천)
// - 연결 정보(공급자·주소·형식)는 여기, 모델 목록은 앱에 내장한 카탈로그(src-tauri/resources/info/default_*_models.json)에서 가져온다.
//   카탈로그는 사람이 고치는 파일이라 모델이 새로 나오면 그 JSON만 고치면 된다 (이 파일은 그대로).
// - formatType은 백엔드 adapter가 고르는 기준 (openai / anthropic / gemini / vertexai)
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import textCatalog from "../../../src-tauri/resources/info/default_text_models.json";
import embeddingCatalog from "../../../src-tauri/resources/info/default_embedding_models.json";

const tr = (key: string) => get(_)(key);

/** 목록에 보이는 모델 하나: 사람이 읽는 이름과 API에 보내는 id */
export interface ModelOption {
	label: string;
	modelId: string;
}
export interface ModelGroup {
	label: string;
	model: ModelOption[];
}
export interface Provider {
	label: string;
	provider: string;
	formatType: string;
	baseUrl: string;
	modelList: ModelGroup[];
}

interface CatalogProvider {
	label: string;
	provider: string;
	modelList: { label: string; model: { label: string; modelId: string; metadata?: TextModelMeta }[] }[];
}

/** 카탈로그에서 공급자(대소문자 무시)의 모델 묶음을 꺼낸다. 없으면 빈 목록 */
function groupsOf(catalog: { data: CatalogProvider[] }, provider: string): ModelGroup[] {
	const p = catalog.data.find((x) => x.provider.toLowerCase() === provider.toLowerCase());
	return (p?.modelList ?? []).map((g) => ({ label: g.label, model: g.model.map((m) => ({ label: m.label, modelId: m.modelId })) }));
}
/** 이름 없이 id만 있는 목록 (음성·그림 모델처럼 카탈로그에 없는 것) */
const ids = (labelKey: string, list: string[]): ModelGroup[] => [{ get label() { return tr(labelKey); }, model: list.map((m) => ({ label: m, modelId: m })) }];

export const apiLists: Provider[] = [
	{ label: "OpenAI", provider: "OpenAI", formatType: "openai", baseUrl: "https://api.openai.com/v1", modelList: groupsOf(textCatalog, "openai") },
	{ label: "Claude Model", provider: "Anthropic", formatType: "anthropic", baseUrl: "https://api.anthropic.com", modelList: groupsOf(textCatalog, "anthropic") },
	// formatType은 백엔드가 아는 "gemini". 예전에 "google"로 저장된 연결도 백엔드가 같은 것으로 받는다
	{ label: "Google Model", provider: "google", formatType: "gemini", baseUrl: "https://generativelanguage.googleapis.com/v1beta", modelList: groupsOf(textCatalog, "google") },
	{ label: "Google VertexAI", provider: "VertexAI", formatType: "vertexai", baseUrl: "", modelList: groupsOf(textCatalog, "vertexai") },
	{ label: "OpenRouter", provider: "openrouter", formatType: "openai", baseUrl: "https://openrouter.ai/api/v1", modelList: [] },
	{ label: "DeepSeek", provider: "deepseek", formatType: "openai", baseUrl: "https://api.deepseek.com/", modelList: groupsOf(textCatalog, "deepseek") },
	// ---- 코어 플러그인 전용 (채팅에는 못 씀): 음성 읽기 · 이미지 생성 ----
	{
		get label() { return tr("settings.extra.elevenlabsLabel"); },
		provider: "elevenlabs",
		formatType: "elevenlabs",
		baseUrl: "https://api.elevenlabs.io/v1",
		modelList: ids("settings.extra.voiceModelsGroup", ["eleven_multilingual_v2", "eleven_flash_v2_5", "eleven_turbo_v2_5", "eleven_v3"]),
	},
	{
		get label() { return tr("settings.extra.novelaiLabel"); },
		provider: "novelai",
		formatType: "novelai",
		baseUrl: "https://image.novelai.net",
		modelList: ids("settings.extra.imageModelsGroup", ["nai-diffusion-4-5-full", "nai-diffusion-4-5-curated", "nai-diffusion-4-full", "nai-diffusion-4-curated-preview", "nai-diffusion-3"]),
	},
];

/** 모델 목록 JSON에 적어 둔 모델 정보 (적은 것만 있음). 백엔드 adapter/catalog.rs와 같은 모양 */
export interface TextModelMeta {
	contextWindow?: number;
	maxOutputTokens?: number;
	inputModalities?: string[];
	/** tools · structuredOutput · reasoning */
	features?: string[];
	/** 추론 강도를 조절할 수 있는 모델: 고를 수 있는 값(낮은 것부터)과 공급자 기본값 */
	reasoning?: { param?: string; levels: string[]; default?: string };
	/** 100만 토큰당 달러 */
	pricing?: { input: number; output: number; long?: { above: number; input: number; output: number } };
}
const metaById = new Map<string, TextModelMeta>();
for (const p of (textCatalog as { data: CatalogProvider[] }).data)
	for (const g of p.modelList) for (const m of g.model) if (m.metadata && !metaById.has(m.modelId.toLowerCase())) metaById.set(m.modelId.toLowerCase(), m.metadata as TextModelMeta);
/** 모델 id로 모델 정보를 찾는다 (정확히 같은 id만, 대소문자 무시). 적어 두지 않았으면 undefined */
export const modelMeta = (modelId: string | null | undefined): TextModelMeta | undefined => (modelId ? metaById.get(modelId.trim().toLowerCase()) : undefined);
/** 추론 강도를 조절할 수 있는 모델이면 그 정보 */
export const reasoningOf = (modelId: string | null | undefined) => {
	const r = modelMeta(modelId)?.reasoning;
	return r?.levels?.length ? r : undefined;
};

/** 그 공급자의 모델 id 전부 (묶음 순서대로) */
export const modelIdsOf = (p: Provider | undefined): string[] => p?.modelList.flatMap((g) => g.model.map((m) => m.modelId)) ?? [];
/** 모델 id → 사람이 읽는 이름 (카탈로그에 없으면 id 그대로) */
export function modelLabel(p: Provider | undefined, id: string): string {
	for (const g of p?.modelList ?? []) for (const m of g.model) if (m.modelId === id) return m.label;
	return id;
}
/** 목록에서 첫 모델 id (새 연결의 기본값) */
export const firstModel = (p: Provider | undefined): string => p?.modelList[0]?.model[0]?.modelId ?? "";

// ---------------------------------------------------------
// 임베딩 모델 (설정 > 메모리 검색): 이름과 기본 차원 같은 정보
// ---------------------------------------------------------
export interface EmbeddingModel extends ModelOption {
	/** 공급자 키 (openai · google · cohere · voyage · mistral) */
	provider: string;
	/** 기본 차원 */
	dimensions: number | null;
	/** 차원을 줄일 수 있는가 */
	adjustable: boolean;
	/** 줄일 수 있는 값들 (정해져 있을 때) */
	supportedDimensions: number[] | null;
	maxInputTokens: number | null;
	/** 100만 토큰당 가격(달러) — 알려진 것만 */
	price: number | null;
}

interface RawEmbedding {
	label: string;
	modelId: string;
	metadata?: {
		dimensions?: { default?: number | null; supported?: number[] | null; adjustable?: boolean };
		maxInputTokens?: number | null;
		pricing?: { input?: number | null };
	};
}

export const embeddingModels: EmbeddingModel[] = (embeddingCatalog.data as { provider: string; modelList: { model: RawEmbedding[] }[] }[]).flatMap((p) =>
	p.modelList.flatMap((g) =>
		g.model.map((m) => ({
			provider: p.provider.toLowerCase(),
			label: m.label,
			modelId: m.modelId,
			dimensions: m.metadata?.dimensions?.default ?? null,
			adjustable: m.metadata?.dimensions?.adjustable === true,
			supportedDimensions: m.metadata?.dimensions?.supported ?? null,
			maxInputTokens: m.metadata?.maxInputTokens ?? null,
			price: m.metadata?.pricing?.input ?? null,
		})),
	),
);

/** 이 연결(공급자 이름)에서 쓸 만한 임베딩 모델. 같은 공급자의 것이 없으면(OpenRouter 등) 전부 */
export function embeddingModelsFor(provider: string | undefined): EmbeddingModel[] {
	const key = (provider ?? "").toLowerCase();
	const own = embeddingModels.filter((m) => m.provider === key);
	return own.length ? own : embeddingModels;
}
export const embeddingModelInfo = (modelId: string): EmbeddingModel | undefined => embeddingModels.find((m) => m.modelId === modelId);
