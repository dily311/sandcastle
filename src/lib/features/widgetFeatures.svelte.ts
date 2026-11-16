// 위젯 하위 기능(위젯 창 열기·마스코트·음악)의 화면 쪽 실행기 (1-D-4: 예전 이름 feautreManager.svelte.ts).
// 앱 전체 기능 켜짐/꺼짐(코어 플러그인)은 백엔드 sys/plugins.rs + src/shared/stores/plugins.svelte.ts가 맡는다 — 여기는 위젯 창 안의 부가 기능만.
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { FeatureRuntime } from "./types";
import { registerWidgetMascot } from "$/lib/features/impl/widget/mascot/mascotFeature.svelte";
import { registerWidgetMusic } from "$/lib/features/impl/widget/music/musicFeature.svelte";
import { registerWidgetWindowFeature } from "./impl/main/widgetWindowFeature.svelte";
import { commands } from "$ipc/bindings";
import { onEvent } from "$shared/ipc/events";

/** 윈도우 별로 사용할 feature 구성 정의 */
const featuresByWindow: Record<string, FeatureRuntime[]> = {
	main: [registerWidgetWindowFeature],
	widget: [registerWidgetMascot, registerWidgetMusic],
};

/**기능 초기화 하는 함수 */
export async function bootstrapWidgetFeatures() {
	// 현재 웹뷰 라벨 가져오기
	const label = getCurrentWindow().label;
	// 설정 불러오기
	await widgetFeatureConfig.loadConfig();
	// 초기화
	await widgetFeatures.initAll(featuresByWindow[label]);
	console.log("[widgetFeatures]", widgetFeatures.getFeatures.filter((f) => f.enabled));
}

class WidgetFeatureRuntime {
	private features = $state.raw<FeatureRuntime[]>([]);

	// 앱 시작시 파일 불러오기
	async initAll(registeredFeatures: FeatureRuntime[]) {
		for (const feature of registeredFeatures) {
			// 초기화 로직 진행
			const configEnabled = widgetFeatureConfig.value.get(feature.id);
			const isEnabled = configEnabled ?? feature.enabled;
			feature.enabled = isEnabled;

			this.register(feature);

			if (configEnabled && isEnabled) {
				await feature.init?.();
			}
		}

		// 리스너 등록
		await onEvent<{ id: string; enabled: boolean }>("feature_setting::changed", widgetFeatureConfig.changeConfig);
	}

	// 기능 등록
	async register(f: FeatureRuntime) {
		if (this.features.some((x) => x.id === f.id)) return; // 중복 방지
		this.features = [...this.features, f];
		// 기본 값 등록
		if (widgetFeatureConfig.value.has(f.id)) return;
		await widgetFeatureConfig.updateConfig(f.id, f.enabled);
	}

	// 기능 제거
	unregister(id: string) {
		this.features.filter((f) => f.id !== id);
	}

	// 기능 토글
	async toggleFeature(id: string, enabled: boolean) {
		const isConfig = widgetFeatureConfig.value.has(id);
		// config에 없으면 반영 X
		if (!isConfig) return;
		// 같은 값 제외
		if ((widgetFeatureConfig.value.get(id) ?? false) === enabled) return;

		// 업데이트
		await widgetFeatureConfig.updateConfig(id, enabled);

		// 현재 기능에 포함되어 있는지 확인
		const feature = this.features.find((f) => f.id === id);
		// 없으면 리턴
		if (!feature) return;

		feature.enabled = enabled;
		if (enabled) {
			await feature.init?.();
		} else {
			feature.destroy?.();
		}
	}

	get getFeatures() {
		return this.features;
	}
}

export const widgetFeatures = new WidgetFeatureRuntime();

class WidgetFeatureConfig {
	private config: Map<string, boolean> = new Map();
	get value() {
		return this.config;
	}

	async loadConfig() {
		const savedConfig = await commands.getFeatureSetting();
		this.config = new Map(Object.entries(savedConfig.feature));
		console.log(this.config);
	}

	async updateConfig(id: string, enabled: boolean) {
		this.value?.set(id, enabled);
		await commands.updateFeatureSetting(id, enabled);
	}

	changeConfig = async(changed: {id: string, enabled: boolean}) => {
		this.config.set(changed.id, changed.enabled);
		const feature = widgetFeatures.getFeatures.find((f)=> f.id === changed.id);
		// 변화없이 동일한 값이 들어왔을 경우
		if (!feature || feature?.enabled === changed.enabled) return;
		feature.enabled = changed.enabled;

		if (changed.enabled) {
			feature.init?.();
		} else {
			feature.destroy();
		}

	}
}
export const widgetFeatureConfig = new WidgetFeatureConfig();
