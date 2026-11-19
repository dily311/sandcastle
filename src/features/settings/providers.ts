// AI 공급자 목록 (설정 > API 키의 "공급자" 고르기, 모델 이름 추천)
// - 연결 정보(공급자·주소·형식)는 여기, 모델 목록은 앱에 내장한 카탈로그(src-tauri/resources/info/default_*_models.json)에서 가져온다.
//   카탈로그는 사람이 고치는 파일이라 모델이 새로 나오면 그 JSON만 고치면 된다 (이 파일은 그대로).
// - formatType은 백엔드 adapter가 고르는 기준 (openai / anthropic / gemini / vertexai)
import textCatalog from "../../../src-tauri/resources/info/default_text_models.json";
import embeddingCatalog from "../../../src-tauri/resources/info/default_embedding_models.json";

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
	modelList: { label: string; model: { label: string; modelId: string; metadata?: unknown }[] }[];
}

/** 카탈로그에서 공급자(대소문자 무시)의 모델 묶음을 꺼낸다. 없으면 빈 목록 */
function groupsOf(catalog: { data: CatalogProvider[] }, provider: string): ModelGroup[] {
	const p = catalog.data.find((x) => x.provider.toLowerCase() === provider.toLowerCase());
	return (p?.modelList ?? []).map((g) => ({ label: g.label, model: g.model.map((m) => ({ label: m.label, modelId: m.modelId })) }));
}
/** 이름 없이 id만 있는 목록 (음성·그림 모델처럼 카탈로그에 없는 것) */
const ids = (label: string, list: string[]): ModelGroup[] => [{ label, model: list.map((m) => ({ label: m, modelId: m })) }];

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
		label: "ElevenLabs (음성)",
		provider: "elevenlabs",
		formatType: "elevenlabs",
		baseUrl: "https://api.elevenlabs.io/v1",
		modelList: ids("음성 모델", ["eleven_multilingual_v2", "eleven_flash_v2_5", "eleven_turbo_v2_5", "eleven_v3"]),
	},
	{
		label: "NovelAI (이미지)",
		provider: "novelai",
		formatType: "novelai",
		baseUrl: "https://image.novelai.net",
		modelList: ids("이미지 모델", ["nai-diffusion-4-5-full", "nai-diffusion-4-5-curated", "nai-diffusion-4-full", "nai-diffusion-4-curated-preview", "nai-diffusion-3"]),
	},
];

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
