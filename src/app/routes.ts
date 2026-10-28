import { pop, push, type RouteDefinition } from "svelte-spa-router";
import ChatScreen from "$screens/chat/ChatDetailPage.svelte";
import SettingsScreen from "$screens/settings/SettingsPage.svelte";
import HomePage from "$screens/HomePage.svelte";
import APIPage from "$screens/settings/APIPage.svelte";
import CharacterListPage from "$screens/character/CharacterListPage.svelte";
import PresetPage from "$screens/preset/PresetPage.svelte";
import ChatListPage from "$screens/chat/ChatListPage.svelte";
import CharacterDetailPage from "$screens/character/CharacterDetailPage.svelte";
import PersonaPage from "$screens/persona/PersonaPage.svelte";
import WorldInfoPage from "$screens/worldinfo/WorldinfoPage.svelte";
import CharacterEditPageSvelte from "$screens/character/CharacterEditPage.svelte";
import GamePage from "$screens/game/GamePage.svelte";
import GameDetailPage from "$/screens/game/GameDetailPage.svelte";
import GameSessionList from "$/screens/game/editor/GameSessionList.svelte";
import GamePlaySession from "$/screens/game/editor/GamePlaySession.svelte";
import GameEditorPage from "$/screens/game/GameEditorPage.svelte";
import GamePlayContainer from "$/screens/game/GamePlayContainer.svelte";

export type Pages = 
	"/" |
	"/api" |
	"/preset" |
	"/character" |
	"/persona" |
	"/chat" |
	"/game" |
	"/worldInfo" |
	"/settings"
;
// Wildcard 경로(/*) 및 동적 파라미터 경로까지 허용하는 라우트 타입
export type RouteKeys = Pages | `${Pages}/*` | `${Pages}/:${string}`;

export interface link {
	title: string;
	desc: string;
	color: string;
	href: Pages;
	icon: string;
}

interface PageItem {
	id: string;
	link: Pages;
	label: string;
	icon: string;
}

export const routes: RouteDefinition = {
	"/": HomePage,
	"/character": CharacterListPage,
	"/character/:characterId": CharacterDetailPage,
	"/character/:characterId/details": CharacterDetailPage,
	"/character/:characterId/chat": ChatListPage,
	"/character/:characterId/edit": CharacterEditPageSvelte,
	"/chat/:conversationId": ChatScreen,
	"/game": GamePage,
	"/game/:gameId": GameDetailPage,
	"/game/:gameId/sessions": GameSessionList,
	"/game/:gameId/play": GamePlaySession,
	"/game/:gameId/play/:sessionId": GamePlayContainer,
	"/game/:gameId/editor": GameEditorPage,
	"/api": APIPage,
	"/preset": PresetPage,
	"/persona": PersonaPage,
	"/worldInfo": WorldInfoPage,
	"/settings": SettingsScreen,
};


export const navigation: PageItem[] = [
	{
		id: "home",
		link: "/",
		label: "홈",
		icon: "House",
	},
	{
		id: "api",
		link: "/api",
		label: "API",
		icon: "Zap",
	},
	{
		id: "preset",
		link: "/preset",
		label: "프리셋",
		icon: "FileText",
	},
	{
		id: "character",
		link: "/character",
		label: "캐릭터",
		icon: "Users",
	},
	{
		id: "game",
		link: "/game",
		label: "게임",
		icon: "Gamepad",
	},
	{
		id: "worldInfo",
		link: "/worldInfo",
		label: "월드인포",
		icon: "Globe",
	},
	{
		id: "persona",
		link: "/persona",
		label: "페르소나",
		icon: "UserCircle",
	},
	{
		id: "settings",
		link: "/settings",
		label: "설정",
		icon: "Settings",
	},
];

// ---------------------------------------------------------
// 사용 함수
// ---------------------------------------------------------

export function handleGoBack() { pop() };
export function handleOpenCharacterDetailPage(id: string) {
	push(`/character/${id}`);
}
export function handleOpenCharacterEditor(id: string) {
	push(`/character/${id}/editor`);
}
export function handleOpenChatPage(id: string) {
	push(`/chat/${id}`);
}
export function handleOpenGameDetailPage(id: string) {
	push(`/game/${id}`);
}
export function handleStartGamePlay(id: string) {
	push(`/game/${id}/sessions`);
}
export function handleOpenGameEditor(id: string) {
	push(`/game/${id}/editor`);
}
