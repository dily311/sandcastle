<script lang="ts">
	import { defaultThemeSetting, themeSetting } from "$/lib/stores/themeStore.svelte";
	import ThemeGallery from "$screens/settings/ThemeGallery.svelte";
	import Button from "$/components/ui/button/Button.svelte";
	import ButtonGroup from "$/components/ui/button/ButtonGroup.svelte";
	import Icon from "$/components/ui/Icon.svelte";
	import IconButton from "$/components/ui/button/IconButton.svelte";
	import ToggleButton from "$/components/ui/button/ToggleButton.svelte";
</script>

<div>
	<div class="setting-card">
		<header class="card-header">
			<h3 class="card-title">테마 및 외관 설정</h3>
		</header>
		<div class="card-body">
			<!-- 컬러 모드 -->
			<div class="setting-row">
				<div class="setting-info">
					<div class="setting-title">컬러 모드</div>
					<p class="description">화면의 기본 색상 모드를 결정합니다.</p>
				</div>
				<div class="setting-control">
					<ButtonGroup segmented={true}>
						<IconButton
							class={themeSetting.mode === "light" ? "on" : ""}
							onclick={() => themeSetting.update({ mode: "light" })}
							title="라이트 모드"
						>
							<Icon name="Sun" />
						</IconButton>
						<IconButton
							class={themeSetting.mode === "dark" ? "on" : ""}
							onclick={() => themeSetting.update({ mode: "dark" })}
							title="다크 모드"
						>
							<Icon name="Moon" />
						</IconButton>
						<IconButton
							class={themeSetting.mode === "system" ? "on" : ""}
							onclick={() => themeSetting.update({ mode: "system" })}
							title="시스템 설정에 맞춤"
						>
							<Icon name="Monitor" />
						</IconButton>
					</ButtonGroup>
				</div>
			</div>

			<!-- 커스텀 CSS 테마 -->
			<div class="setting-row">
				<div class="setting-info">
					<div class="setting-title">커스텀 CSS 테마</div>
					<p class="description">등록된 테마를 적용하거나 새로운 CSS 테마를 추가/관리합니다.</p>
				</div>
				<div class="setting-control group">
					<select
						class="custom-select"
						value={themeSetting.selectTheme}
						onchange={(e) => themeSetting.update({ selectTheme: e.currentTarget.value })}
					>
						<option value="default">기본</option>
						{#each themeSetting.cssThemeValue as item}
							<option value={item.filename}>{item.name}</option>
						{/each}
					</select>
				</div>
			</div>

			<!-- 프레임리스 설정 -->
			<div class="setting-row">
				<div class="setting-info">
					<span class="setting-title">프레임리스 윈도우</span>
					<p class="description">창 타이틀바 테두리를 제거하고 커스텀 프레임을 사용합니다.</p>
				</div>
				<div class="setting-control">
					<ToggleButton
						checked={themeSetting.isFrameless}
						toggle={(e, change) => themeSetting.update({ isFrameless: change })}
					/>
				</div>
			</div>

			<!-- Accent Color -->
			<div class="setting-row">
				<div class="setting-info">
					<span class="setting-title">강조 색상 (Accent Color)</span>
					<p class="description">버튼, 활성 아이템 등에 사용될 강조 포인트를 지정합니다.</p>
				</div>
				<div class="setting-control color-picker-group">
					<input
						type="color"
						class="color-picker"
						value={themeSetting.accentColor}
						onchange={(e) => themeSetting.update({ accentColor: e.currentTarget?.value })}
					/>
					<span class="color-hex">{themeSetting.accentColor}</span>
					<IconButton
						title="기본색상으로 되돌리기"
						disabled={themeSetting.accentColor === defaultThemeSetting.accentColor}
						onclick={() => themeSetting.update({ accentColor: defaultThemeSetting.accentColor })}
					>
						<Icon name="RotateCcw" />
					</IconButton>
				</div>
			</div>

			<!-- Font Size -->
			<div class="setting-row">
				<div class="setting-info">
					<span class="setting-title">글꼴 크기</span>
					<p class="description">앱 전체 기본 폰트 크기를 조정합니다.</p>
				</div>
				<div class="setting-control slider-group">
					<input
						type="range"
						min="12"
						max="24"
						step="1"
						class="range-slider"
						value={themeSetting.fontSize}
						onchange={(e) => themeSetting.update({ fontSize: Number(e.currentTarget.value) })}
					/>
					<span class="slider-value">{themeSetting.fontSize}px</span>
					<IconButton
						title="기본크기로 되돌리기"
						disabled={themeSetting.fontSize === defaultThemeSetting.fontSize}
						onclick={() => themeSetting.update({ fontSize: defaultThemeSetting.fontSize })}
					>
						<Icon name="RotateCcw" />
					</IconButton>
				</div>
			</div>
		</div>
	</div>
	<div class="setting-card">
		<ThemeGallery />
	</div>
</div>
