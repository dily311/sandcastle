<script lang="ts">
	// 플러그인이 manifest의 `settings`로 선언한 설정칸을 앱이 그려 준다 (6-D-8). 값은 바꾸는 즉시 저장,
	// 플러그인은 sandcastle.settings.get()으로 읽는다 (저장하면 호스트가 플러그인을 다시 시작해 새 값을 읽게 함)
	import type { PluginSettingField } from "$ipc/bindings";
	import { Field, Input, Select, Switch } from "$shared/ui";
	import { admin } from "./admin.svelte";

	let { pluginId, fields }: { pluginId: string; fields: PluginSettingField[] } = $props();
	let values = $state<Record<string, unknown>>({});

	$effect(() => {
		void pluginId;
		void admin.settings(pluginId).then((v) => (values = v));
	});

	async function save(key: string, value: unknown) {
		const next = await admin.setSetting(pluginId, key, value);
		if (next) values = next;
		else values = await admin.settings(pluginId); // 거절당했으면 저장된 값으로 되돌림
	}
</script>

<p class="warn">여기 적은 값은 이 플러그인이 읽을 수 있습니다. 비밀번호나 API 키는 넣지 마세요.</p>
{#each fields as f (f.key)}
	{#if f.type === "toggle"}
		<Switch checked={values[f.key] === true} label={f.label} description={f.description ?? undefined} onchange={(v) => save(f.key, v)} />
	{:else}
		<Field label={f.label} hint={f.description ?? undefined} for={`ps-${pluginId}-${f.key}`}>
			{#if f.type === "select"}
				<Select id={`ps-${pluginId}-${f.key}`} value={String(values[f.key] ?? "")} options={(f.options ?? []).map((o) => ({ value: o, label: o }))} onchange={(e) => save(f.key, e.currentTarget.value)} />
			{:else if f.type === "number"}
				<Input id={`ps-${pluginId}-${f.key}`} type="number" value={Number(values[f.key] ?? 0)} onchange={(e) => save(f.key, Number(e.currentTarget.value))} />
			{:else}
				<Input id={`ps-${pluginId}-${f.key}`} value={String(values[f.key] ?? "")} onchange={(e) => save(f.key, e.currentTarget.value)} />
			{/if}
		</Field>
	{/if}
{/each}

<style>
	.warn {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
</style>
