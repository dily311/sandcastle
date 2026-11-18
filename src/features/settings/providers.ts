// AI 공급자 목록 (설정 > API 키의 "공급자" 고르기, 모델 이름 추천)
// formatType은 백엔드 adapter가 고르는 기준 (openai / anthropic / google / vertexai)
export interface Provider {
	label: string;
	provider: string;
	formatType: string;
	baseUrl: string;
	modelList: { label: string; model: string[] }[];
}

export const apiLists: Provider[] = [
    {
        label: "OpenAI",
        provider: "OpenAI",
        formatType: "openai",
        baseUrl: "https://api.openai.com/v1",
        modelList: [
            {
                label: "gpt-5.6",
                model: [
                    "gpt-5.6-sol",
                    "gpt-5.6-terra",
                    "gpt-5.6-luna",
                ]
            }
        ]
    },
    {
        label: "Claude Model",
        provider: "Anthropic",
        formatType: "anthropic",
        baseUrl: "https://api.anthropic.com",
        modelList: [
            {
                label: "Claude 5",
                model: [
                    "claude-fable-5-2",
                    "claude-opus-5-5",
                    "claude-sonnet-5-5",
                    "claude-haiku-5-5",
                ]
            },
            {
                label: "Versions",
                model: [
                    "claude-opus-4-7",
                    "claude-opus-4-6",
                    "claude-opus-4-5",
                ]
            }
        ]
    },
    {
        label: "Google Model",
        provider: "google",
        formatType: "google",
        baseUrl: "https://generativelanguage.googleapis.com/v1beta",
        modelList: [
            {
                label: "gemini-3.1",
                model: [
                    "gemini-3.1-pro-preview",
                    "gemini-3.7-flash",
                    "gemini-3.5-flash",
                    "gemini-3.1-flash-lite-preview",
                ]
            },
            {
                label: "gemini-2.5",
                model: [
                    "gemini-2.5-pro",
                ]
            }
        ]
    },
    {
        label: "Google VertexAI",
        provider: "VertexAI",
        formatType: "vertexai",
        baseUrl: "",
        modelList: [
            {
                label: "gemini-3.1",
                model: [
                    "gemini-3.1-pro-preview",
                    "gemini-3.7-flash",
                    "gemini-3.5-flash",
                    "gemini-3.1-flash-lite-preview",
                ]
            },
            {
                label: "gemini-2.5",
                model: [
                    "gemini-2.5-pro",
                ]
            }
        ]
    },
    {
        label: "OpenRouter",
        provider: "openrouter",
        formatType: "openai",
        baseUrl: "https://openrouter.ai/api/v1",
        modelList: [

        ]
    },
    {
        label: "DeepSeek",
        provider: "deepseek",
        formatType: "openai",
        baseUrl: "https://api.deepseek.com/",
        modelList: [
            {
                label: "deepseek",
                model: [
                    "deepseek-v4-pro",
                    "deepseek-v4-flash",
                ]
            }
        ]
    },
    // ---- 코어 플러그인 전용 (채팅에는 못 씀): 음성 읽기 · 이미지 생성 ----
    {
        label: "ElevenLabs (음성)",
        provider: "elevenlabs",
        formatType: "elevenlabs",
        baseUrl: "https://api.elevenlabs.io/v1",
        modelList: [
            {
                label: "음성 모델",
                model: ["eleven_multilingual_v2", "eleven_flash_v2_5", "eleven_turbo_v2_5", "eleven_v3"]
            }
        ]
    },
    {
        label: "NovelAI (이미지)",
        provider: "novelai",
        formatType: "novelai",
        baseUrl: "https://image.novelai.net",
        modelList: [
            {
                label: "이미지 모델",
                model: ["nai-diffusion-4-5-full", "nai-diffusion-4-5-curated", "nai-diffusion-4-full", "nai-diffusion-4-curated-preview", "nai-diffusion-3"]
            }
        ]
    }
]
