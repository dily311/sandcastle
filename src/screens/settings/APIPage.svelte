<script lang="ts">
	import { apiLists } from "$/data/aiModelList";
	import { APICommands } from "$/ipc/commands";
	import { apiProfiles } from "$/lib/stores/apiProfileStore.svelte";
	import { appSetting } from "$/lib/stores/settingStore.svelte";
	import type { APIList, ApiProfile, ApiProfileInput } from "$type/aiService";
	import { Plus, Check, MoreVertical, Zap } from "@lucide/svelte";
	import { onMount } from "svelte";

	let isLoading = $state(false);

	let apiKey = $state("");
	let provider = $state("OpenAI");
	let selectedProvider = $derived(apiLists.find((value) => value.provider === provider));
	let apiEndpoint = $state("");
	let selectedModel = $state("");

	$effect(() => {
		if (selectedProvider) {
			apiEndpoint = selectedProvider.baseUrl ?? "";
			selectedModel = selectedProvider.modelList?.[0]?.model?.[0] ?? "";
		}
	});

	async function handleSave() {
		if (!selectedProvider || !apiEndpoint || !selectedModel) return;
		const input: ApiProfileInput = {
			name: selectedProvider?.label,
			provider: selectedProvider?.provider,
			baseUrl: apiEndpoint,
			formatType: selectedProvider?.formatType,
			model: selectedModel,
			// TODO: app 설정엣서 가져와서 넘기도록 하던가 아니면 백엔드에서 처리하게 수정하면 될듯?
			selectedPresetId: null,
		};
		const profile = await apiProfiles.create(input, apiKey);
		await appSetting.update({ selectedApiProfile: profile.id });
	}

	async function handleTest() {
		isLoading = true;
		const selectedApiProfile = apiProfiles.selectedApiProfile;
		if (!selectedApiProfile) {
			return console.log("api 프로필이 선택되지 않음");
		}
		const responseText = await APICommands.sendMessages({
			profileId: selectedApiProfile.id,
			conversationId: "",
			message: "",

		});
		console.log(responseText);

		isLoading = false;
	}

	async function getModels() {
		isLoading = true;
		const selectedApiProfile = apiProfiles.selectedApiProfile;
		if (!selectedApiProfile) {
			return console.log("api 프로필이 선택되지 않음");
		}
		const models = await APICommands.getModels({ profileId: selectedApiProfile.id });
		console.log(models);
		isLoading = false;
	}

	async function selectedApiProfile(profile: ApiProfile) {
		await appSetting.update({ selectedApiProfile: profile.id });
		provider = profile.provider;
		apiEndpoint = profile.baseUrl;
		selectedModel = profile.model;
	}
	onMount(() => {
		const selectedApiProfile = apiProfiles.selectedApiProfile;
		if (!selectedApiProfile) {
			provider = "OpenAI";
			apiEndpoint = selectedProvider?.baseUrl ?? "";
			selectedModel = selectedProvider?.modelList[0].model[0] ?? "";
		} else {
			provider = selectedApiProfile.provider;
			apiEndpoint = selectedApiProfile.baseUrl;
			selectedModel = selectedApiProfile.model;
		}
	});
</script>

<div class="page">
	<div class="page-title">
		<div>
			<h2>AI API 연결</h2>
			<p>AI와 대화하기 위해 사용할 API 연결을 관리합니다.</p>
		</div>

		<button class="primary">
			<Plus size={16} />
			연결 추가
		</button>
	</div>

	<div class="layout">
		<section class="panel">
			<div class="panel-header">
				<div>
					<h3>API 프로필</h3>
					<p>현재 저장된 API 목록입니다.</p>
				</div>
			</div>

			<div class="connection-list">
				{#each apiProfiles.value as item}
					<button
						class:connected={appSetting.selectedApiProfile === item.id}
						class="connection"
						onclick={() => selectedApiProfile(item)}
					>
						<div class="provider-icon">
							<Zap size={18} />
						</div>

						<div class="connection-info">
							<strong>{item.name}</strong>
							<span>{item.model}</span>
						</div>

						{#if apiProfiles.isConnected}
							<span class="status">
								<span></span>
								연결됨
							</span>
						{/if}

						<MoreVertical size={17} />
					</button>
				{/each}
			</div>
		</section>

		<section class="panel editor">
			<div class="panel-header">
				<div>
					<h3>연결 설정</h3>
					<p>OpenAI API를 사용합니다.</p>
				</div>

				<span class="badge">OpenAI</span>
			</div>

			<div class="form">
				<label>
					<span>Provider</span>
					<select bind:value={provider}>
						{#each apiLists as item}
							<option value={item.provider}>{item.provider}</option>
						{/each}
					</select>
				</label>

				<label>
					<span>{selectedProvider?.label}</span>
					<select bind:value={selectedModel}>
						{#each selectedProvider?.modelList as item}
							<optgroup label={item.label}>
								{#each item.model as modelName}
									<option value={modelName}>{modelName}</option>
								{/each}
							</optgroup>
						{/each}
					</select>
				</label>

				<label>
					<span>API Key</span>
					<input type="password" bind:value={apiKey} placeholder="여기에 api 키를 입력하세요" />
				</label>

				<label>
					<span>Base URL</span>
					<input bind:value={apiEndpoint} />
				</label>

				<div class="test-row">
					<button class="secondary" onclick={() => getModels()} disabled={isLoading}>모델 불러오기</button>
					<button class="secondary" onclick={() => handleTest()} disabled={isLoading}>
						<Check size={15} />
						연결 테스트
					</button>

					<button class="save-btn" onclick={() => handleSave()} disabled={isLoading}>저장</button>
				</div>
			</div>
		</section>
	</div>
</div>

<style>
	.page-title {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 18px;
	}

	.page-title p {
		margin: 5px 0 0;
		color: var(--text-muted);
		font-size: 11px;
	}

	.primary,
	.save-btn {
		height: 36px;
		padding: 0 13px;

		display: flex;
		align-items: center;
		gap: 6px;

		border: 0;
		border-radius: 8px;

		background: var(--accent-color);
		color: white;

		font-size: 11px;
		cursor: pointer;
	}

	.layout {
		display: grid;
		grid-template-columns: 1fr 1.25fr;
		gap: 14px;
	}

	.panel {
		border: 1px solid var(--card-border-color);
		border-radius: 15px;
		background: var(--card-bg);
		overflow: hidden;
	}

	.panel-header {
		min-height: 65px;
		padding: 15px 17px;

		display: flex;
		align-items: center;
		justify-content: space-between;

		border-bottom: 1px solid var(--border-color);
	}

	.panel-header h3 {
		margin: 0;
		font-size: 13px;
		font-weight: 600;
		color: var(--text-color);
	}

	.panel-header p {
		margin: 4px 0 0;
		font-size: 10px;
		color: var(--text-muted);
	}

	.connection-list {
		padding: 8px;
	}

	.connection {
		width: 100%;
		min-height: 65px;
		padding: 10px;

		display: flex;
		align-items: center;
		gap: 10px;

		border: 1px solid transparent;
		border-radius: 10px;

		background: transparent;
		color: var(--text-muted);

		text-align: left;
		cursor: pointer;
	}

	.connection:hover,
	.connection.connected {
		border-color: var(--card-border-color);
		background: var(--card-hover-bg);
	}

	.provider-icon {
		width: 37px;
		height: 37px;

		display: flex;
		align-items: center;
		justify-content: center;

		border-radius: 10px;
		background: #f0efff;
		color: var(--accent-color);
	}

	.connection-info {
		flex: 1;
	}

	.connection-info strong {
		display: block;
		font-size: 12px;
		color: var(--text-color);
	}

	.connection-info span {
		display: block;
		margin-top: 3px;
		font-size: 10px;
		color: #9a9ea7;
	}

	.status {
		display: flex !important;
		align-items: center;
		gap: 5px;

		color: var(--color-green) !important;
	}

	.status > span {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-green);
	}

	.editor {
		min-height: 420px;
	}

	.badge {
		padding: 5px 8px;
		border-radius: 6px;

		background: var(--badge-color);
		color: var(--accent-color);
		font-size: 10px;
	}

	.form {
		padding: 18px;
	}

	label {
		display: block;
		margin-bottom: 15px;
	}

	label > span {
		display: block;
		margin-bottom: 6px;
		font-size: 11px;
		color: var(--text-muted);
	}

	input,
	select {
		width: 100%;
		height: 38px;
		color: var(--text-color);
		font-size: 11px;
	}

	.test-row {
		margin-top: 23px;

		display: flex;
		justify-content: flex-end;
		gap: 7px;
	}

	.secondary {
		height: 36px;
		padding: 0 12px;

		display: flex;
		align-items: center;
		gap: 6px;

		border: 1px solid var(--border-color);
		border-radius: 8px;

		background: white;
		color: var(--text-faint);

		font-size: 11px;
		cursor: pointer;
	}
</style>
