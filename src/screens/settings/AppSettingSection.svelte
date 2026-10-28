<script lang="ts">
	import ToggleButton from "$/components/ui/button/ToggleButton.svelte";
	import { appSetting } from "$/lib/stores/settingStore.svelte";
	import { type Language } from "$/lib/type/settingTypes";
</script>

<div>
	<div class="setting-card">
		<header class="card-header">
			<h3 class="card-title">기본 설정</h3>
		</header>
		<div class="card-body">
			<!-- 언어 -->
			<div class="setting-row">
				<div class="setting-info">
					<span class="label">언어</span>
					<p class="description">애플리케이션 표시 언어를 지정합니다.</p>
				</div>
				<div class="setting-control">
					<select
						class="custom-select"
						value={appSetting.language}
						onchange={(e) => appSetting.update({ language: e.currentTarget.value as Language })}
					>
						<option value="ko">한국어</option>
						<option value="en">English</option>
					</select>
				</div>
			</div>

			<!-- 자동 업데이트 -->
			<div class="setting-row">
				<div class="setting-info">
					<span class="label">자동 업데이트</span>
					<p class="description">새 버전이 출시되면 자동으로 다운로드 및 설치합니다.</p>
				</div>
				<div class="setting-control">
					<ToggleButton
						checked={appSetting.autoUpdate}
						toggle={(e, change) => appSetting.update({ autoUpdate: change })}
					/>
				</div>
			</div>
		</div>
	</div>
	<div class="setting-card">
		<header class="card-header">
			<h3 class="card-title">채팅 설정</h3>
		</header>
		<div class="card-body">
			<!-- Enter 전송 -->
			<div class="setting-row">
				<div class="setting-info">
					<span class="label">Enter 키로 메시지 전송</span>
					<p class="description">비활성화 시 Shift + Enter 동작과 전환됩니다.</p>
				</div>
				<div class="setting-control">
					<ToggleButton
						checked={appSetting.sendOnEnter}
						toggle={(e, change) => appSetting.update({ sendOnEnter: change })}
					/>
				</div>
			</div>

			<!-- 타임스탬프 표시 -->
			<div class="setting-row">
				<div class="setting-info">
					<span class="label">타임스탬프 표시</span>
					<p class="description">메시지 옆에 작성 시간을 항상 표시합니다.</p>
				</div>
				<div class="setting-control">
					<ToggleButton
						checked={appSetting.showTimestamps}
						toggle={(e, change) => appSetting.update({ showTimestamps: change })}
					/>
				</div>
			</div>
		</div>
	</div>
</div>
