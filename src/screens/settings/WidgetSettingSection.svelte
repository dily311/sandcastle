<script lang="ts">
	import { WIDGET_FEATURES } from "$/lib/features/setFeautres";
	import { featureConfig, featureManager } from "$/lib/features/feautreManager.svelte";
	import Icon from "$/components/ui/Icon.svelte";
	import { widgetCommands } from "$/ipc/commands";
	import { onMount } from "svelte";
	import ToggleButton from "$/components/ui/button/ToggleButton.svelte";

	let isAutoStartWidget = $state(false);

	onMount(async () => {
		isAutoStartWidget = await widgetCommands.is_autostart_enabled();
	});
</script>

<div>
	<div>
		<div class="setting-row">
			<div class="setting-info">
				<div class="setting-title">위젯</div>
			</div>
			<div class="setting-control">
				<ToggleButton
					checked={featureConfig.value.get("widget") || false}
					toggle={(e, change) => featureManager.toggleFeature("widget", change)}
				/>
			</div>
		</div>
		<div class="setting-row">
			<div class="setting-info">
				<div class="setting-title">위젯 자동 실행</div>
			</div>
			<div class="setting-control">
				<ToggleButton
					checked={isAutoStartWidget}
					toggle={(e, change) => widgetCommands.set_autostart({ enabled: change })}
				/>
			</div>
		</div>
	</div>
	<div class="setting-card">
		<header class="card-header">
			<h3 class="card-title">위젯 설정</h3>
		</header>
		<div class="card-body">
			{#each WIDGET_FEATURES as feature}
				<div class="setting-row">
					<div class="setting-info">
						<div class="setting-title">
							<div class="setting-icon">
								<Icon name={feature.icon} />
							</div>
							<div class="label">{feature.name}</div>
						</div>
						<div class="description">
							{feature.description}
						</div>
					</div>
					<div class="setting-control">
						<ToggleButton
							checked={featureConfig.value.get(feature.id) || false}
							toggle={(e, change) => featureManager.toggleFeature(feature.id, change)}
						/>
					</div>
				</div>
			{/each}
		</div>
	</div>
</div>

<style>
</style>
