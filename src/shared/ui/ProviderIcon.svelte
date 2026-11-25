<script lang="ts" module>
	// AI 공급자 아이콘. 모델 이름으로 공급자를 짐작한다 (메시지 meta.info.model).
	// 브랜드 아이콘은 simple-icons(CC0)에서 쓰는 것만 가져온다. OpenAI·DeepSeek 로고는 이 버전의 simple-icons에 없어 일반 아이콘으로 대신한다.
	import { siAnthropic, siGooglegemini, siMeta, siMistralai, siOllama, siOpenrouter, siPerplexity, siX } from "simple-icons";

	type Brand = { title: string; path: string };
	const RULES: [RegExp, Brand | "openai"][] = [
		[/claude|anthropic/i, siAnthropic],
		[/gemini|gemma|palm|bison/i, siGooglegemini],
		[/(^|\/)(gpt|o\d|chatgpt|dall-e|text-|davinci)/i, "openai"],
		[/mistral|mixtral|codestral|ministral/i, siMistralai],
		[/llama|meta-/i, siMeta],
		[/grok/i, siX],
		[/sonar|perplexity/i, siPerplexity],
		[/openrouter/i, siOpenrouter],
		[/ollama|qwen|phi-?\d/i, siOllama],
	];

	/** 모델 이름 → 공급자 아이콘 (모르면 null) */
	export function providerOf(model: string | undefined): Brand | "openai" | null {
		if (!model) return null;
		return RULES.find(([re]) => re.test(model))?.[1] ?? null;
	}
</script>

<script lang="ts">
	import { Bot, Sparkles } from "@lucide/svelte";
	import { _ } from "svelte-i18n";

	let { model, size = 14 }: { model: string | undefined; size?: number } = $props();
	const brand = $derived(providerOf(model));
</script>

<span class="provider" title={model ?? "AI"} aria-label={model ? $_("uiPolish.modelAria", { values: { model } }) : "AI"} role="img" style:--size="{size}px">
	{#if brand === "openai"}
		<Sparkles size={size} />
	{:else if brand}
		<svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true"><path d={brand.path} fill="currentColor" /></svg>
	{:else}
		<Bot size={size} />
	{/if}
</span>

<style>
	.provider {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--size);
		height: var(--size);
		color: var(--text-muted);
		cursor: help;
		flex-shrink: 0;
	}
</style>
