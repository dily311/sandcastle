/*! *****************************************************************************
언어 설정 유틸
***************************************************************************** */

import { register, init, getLocaleFromNavigator } from "svelte-i18n";

// 사용할 언어 파일 등록 (번들 크기 최적화를 위해 동적 import 사용)
register("ko", () => import("../../locales/ko.json"));
register("en", () => import("../../locales/en.json"));

init({
	fallbackLocale: "en", // 번역이 없을 때 대체할 기본 언어
	initialLocale: getLocaleFromNavigator() || "en", // 브라우저 언어 감지, 실패시 한국어
});
