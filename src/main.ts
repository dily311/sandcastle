import "$/styles/font.css";
import "$shared/ui/base.css";
import { mount } from "svelte";
import AppShell from "$app/AppShell.svelte";
import { bootstrapFeatures } from "$lib/features/feautreManager.svelte";
import { logs } from "$shared/stores/log.svelte";

// 앱 안에서는 브라우저 기본 우클릭 메뉴를 쓰지 않는다 (필요한 곳은 ContextMenu)
document.addEventListener("contextmenu", (e) => e.preventDefault(), false);

const app = mount(AppShell, { target: document.getElementById("app")! });

// 위젯 창 같은 부가 기능은 화면과 따로 띄운다 — 실패해도 앱은 계속
if (__IS_TAURI__) bootstrapFeatures().catch((e) => logs.error("feature", String(e)));

export default app;
