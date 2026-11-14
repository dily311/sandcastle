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
    }
]
