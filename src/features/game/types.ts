// ! rust로 넘어오는 데이터를 정의
// ! 혼동이 없도록 models/game.rs와 구조체명을 동일하게 할 것

/** 게임 메타 데이터 */
export interface GameProject {
	id: string;
	title: string;
	description: string;
	templateId: string;
	version: string; // 게임버전: semver, 세이브 호환성 체크용
	apiVersion: string;
	coverImage: string | null;
	startSceneKey: string | null;
	startMapId: string | null;
	startLocationId: string | null;
	worldInfoIds: string[];
	customCss: string | null;
	settingsJson: Record<string, unknown> | null;
	createdAt: number;
	updatedAt: number;
}

/** 템플릿 리스트 */
export interface Template {
	id: string;
	name: string;
	apiVersion: string;
	templateVersion: string;
	capabilities: Capabilities;
}

export interface Capabilities {
	choices: boolean;
	freeText: boolean;
	inventory: boolean;
	map: boolean;
	aiNarration: boolean;
	location: boolean;
	time: boolean;
}

// =========================================================
// 게임 데이터 로드
// =========================================================

/**
 * 정적 데이터
 * 게임 데이터 로드시 해당 인터페이스를 불러온다.
 * 그 이후로는 불러온 데이터에서 변경된 상태만 반환된다.
 */
export interface OpenGameResponse {
	project: GameProject;
	view: GameDefinition;
	// view 내용의 해시값. 프론트가 캐시해둔 정적 데이터가 유효한지 비교
	contentHash: string;
	// 에셋을 불러오는 기준
	assetBaseUrl: string;
	// 데이터 진단 결과. 에러나 경고를 표시하는 데 사용
	diagnostics: Issue[];
}

export interface Issue {
	severity: "error" | "warning";
	message: string;
	targets: string[];
	text: string;
}

export interface GameDefinition {
	game: {
		id: string;
		title: string;
		description: string;
		coverImage: string;
		customCss: string;
		settings: string;
	};
	template: string;
	maps: GameMap[];
	zones: Zone[];
	locations: Location[];
	connections: LocationConnection[];
	interactables: InteractTable[];
	characters: Character[];
	timePeriods: TimePeriod[];
	weatherTypes: WeatherType[];
	scenes: Scene[];
	items: Item[];
	relationshipAxes: RELATION_AXES;
	assets: Assets;
	stageCatalog: {
		bgmMoods: BGM_MOODS;
		screenEffects: SCREEN_EFFECTS;
		motions: MOTIONS;
	};
	endings: Ending[];
}

// js의 map과 혼동으로 GameMap으로 변경
export interface GameMap {
	id: string;
	name: string;
	dangerLevel: string;
	settings: any;
}

export interface Zone {
	id: string;
	mapId: string;
	name: string;
	tags: string[];
	settings: any;
}

export interface Location {
	id: string;
	zoneId: string;
	name: string;
	description: string;
	settings: any;
}

export interface LocationConnection {
	fromId: string;
	toId: string;
	travelSeconds: number;
	settings: any;
}

export interface InteractTable {
	id: string;
	locationId: string;
	name: string;
}

export interface Character {
	id: string;
	name: string;
	avatar: string;
	description: string;
	firstMessage: string;
	tags: string[];
	settings: any;
}

export interface TimePeriod {
	id: string;
	name: string;
	startMinute: number;
	endMinute: number;
}
export interface WeatherType {
	id: string;
	name: string;
}

export interface Scene {
	id: string;
	sceneKey: string;
	name: string;
	description: string;
	isEnding: boolean;
	settings: any;
	choices: Choice[];
}

export interface Choice {
	id: string;
	text: string;
	sortOrder: number;
}

export interface Item {
	id: string;
	name: string;
	description: string;
	// svg 경로
	icon: string;
	kind: string;
	maxStack: number;
	isUnique: boolean;
	isKey: boolean;
	consumable: boolean;
	usable: boolean;
	settings: any;
}

// 처음에는 고정
export type RELATION_AXES = "affection" | "trust" | "tension";

export interface Assets {
	audio: AudioAsset[];
	characters: CharacterAsset[];
	cgs: CgsAsset[];
	backgrounds: BackgroundAsset[];
	other: OtherAsset[];
}

export interface AudioAsset {
	id: string;
	key: string;
	category: string;
	path: string;
	settings: any;
	tag: string[];
	characterId: string;
	// 변환한 대사 문자열 보관
	transcript: string;
}

export interface CharacterAsset {
	id: string;
	key: string;
	path: string;
	settings: any;
	characterId: string;
	usage: string;
	emotion: string;
}
export interface CgsAsset {
	id: string;
	key: string;
	path: string;
	settings: any;
	characterId: string;
	description: string;
}
export interface BackgroundAsset {
	id: string;
	key: string;
	path: string;
	settings: any;
	locationId: string;
	timeId: string;
	weatherId: string;
}
export interface OtherAsset {
	id: string;
	key: string;
	category: string;
	path: string;
	settings: any;
}

export type BGM_MOODS = "default" | "peaceful" | "upbeat" | "romance" | "tense" | "battle";
export type SCREEN_EFFECTS =
	| "shake"
	| "flash"
	| "blackout"
	| "fade_in"
	| "fireworks"
	| "petals"
	| "snow"
	| "rain"
	| "fog"
	| "clear";
export type MOTIONS = "jump" | "bounce" | "shake" | "nod" | "sway" | "shrink" | "slide_in" | "slide_out";

export interface Ending {
	id: string;
	endingKey: string;
	title: string;
	description: string;
}

export interface variable {
	key: string;
	description: string;
	type: string;
	default: number;
	min: number;
	max: number;
}

// ---------------------------------------------------------
// // 런타임 시 게임 엔진에서 넘어오는 데이터
// ---------------------------------------------------------
/**
 * 랜더링 데이터
 * 정적 데이터에서 변한 데이터만 반환된다.
 */
export interface ContinueGameResponse {
	// 게임엔진에서 돌아간 이벤트 목록
	// 무시해도 됨
	events: GameEvent[];
	// 화면 표시 연출용
	state: RenderState;
	history: TranscriptPage;
}

/** 변경된 데이터
 * stage에는 현재 출력되어야 하는 데이터가 들어가 있음
 */
export interface RenderState {
	sessionId: string;
	locationId: string | null;
	mapId: string;
	zoneId: string | null;
	gameTime: number;
	weatherId: string;
	timePeriodId: string;
	currentSceneId: string | null;
	activeNpcId: string | null;
	availableChoices: string[];
	availableMoves: AvailableMove[];
	/** 씬 입력(sceneSay)과 씬 끝내기를 쓸 수 있는가 (씬이 열려 있고 엔딩이 아니며 AI 이어가기가 켜져 있음) */
	sceneChat: boolean;
	flags: any;
	inventory: any;
	relationships: Record<string, RelationshipDetail>;
	stage: Stage;
	presentNpcs: string[];
	/** StoryPlan 체크포인트에 도달함 (켜진 게임만) */
	storyPlanDue?: boolean;
}

export interface AvailableMove {
	toId: string;
	travelSeconds: number;
	available: boolean;
}
export interface RelationshipDetail {
	tags: string[];
	affection?: number;
	trust?: number;
	tension?: number;
}


export interface TranscriptPage {
	entries: TranscriptEntry[];
	hasMore: boolean;
	oldestSeq: number | null;
}

export interface TranscriptEntry {
	seq: number;
	gameTime: number;
	kind: "sceneScript" | "message";
	sceneId: string | null;
	role: string | null;
	lines: StageLine[];
}

// ---------------------------------------------------------
// 게임 커맨드와 이벤트 정의
// ---------------------------------------------------------
export type GameCommand =
	| { type: "moveTo"; locationId: string }
	| { type: "interact"; entityId: string }
	| { type: "talk"; gameCharacterId: string }
	| { type: "stopTalk" }
	| { type: "setVariable"; key: string; value: any }
	| { type: "choose"; choiceId: string }
	| { type: "endScene" }
	| { type: "advanceTime"; minutes: number }
	| { type: "useItem"; itemId: string; targetId?: string }
	| { type: "dropItem"; itemId: string; quantity?: number };

/** 프론트에서는 시스템이 주고 받은 이벤트의 결과만 반환됨
 * 디버깅용으로 받는 것이므로 신경쓰지마셈
 */
export type GameEvent =
	| { type: "gameStarted" }
	| { type: "locationChanged"; from?: string; to: string }
	| { type: "zoneChanged"; from?: string; to: string }
	| { type: "timeAdvanceRequested"; seconds: number }
	| { type: "timeAdvanced"; from: number; to: number }
	| { type: "weatherChanged"; weatherId: string }
	| { type: "timePeriodChanged"; from?: string; to: string }
	| { type: "npcMoved"; gameCharacterId: string; from?: string; to?: string }
	| { type: "eventTriggered"; eventId: string; eventType: string; triggerType: string; npcId?: string }
	| { type: "sceneRequested"; sceneId: string }
	| { type: "eventRequested"; sceneId: string }
	| { type: "triggerActivated"; sceneId: string }
	| { type: "dialogueStarted"; sceneId: string }
	| { type: "dialogueEnded"; sceneId: string }
	| { type: "choicePresented"; choices: string[] }
	/** continuesScene: 다음 씬이 없고 끝내는 효과도 없음 → 선택지 문구로 sceneSay를 이어 부른다 */
	| { type: "choiceMade"; choiceId: string; nextSceneId?: string | null; effect?: unknown; continuesScene: boolean }
	| { type: "sceneEndRequested" }
	| { type: "sceneChanged"; sceneId: string }
	| { type: "stageRequested"; spec: any }
	| { type: "sceneScript"; sceneId: string; lines: StageLine[] }
	| { type: "charactersAppeared", characterIds: string[] }
	| { type: "stageLines"; lines: StageLine[] }
	| { type: "sceneExited"; sceneId: string }
	| { type: "variableChanged"; key: string; old: any; new: any }
	| { type: "itemChangeRequested"; itemId: string; delta: number }
	| { type: "itemUsed"; itemId: string; targetId?: string }
	| { type: "relationChangeRequested"; from: string; to: string; axis: string; delta: number }
	| { type: "relationChanged"; from: string; to: string; axis: string; old: number; new: number }
	| { type: "itemAdded"; itemId: string; quantity: number }
	| { type: "itemRemoved"; itemId: string; quantity: number }
	| { type: "gameEnded"; endingId: string };

// ---------------------------------------------------------
// NPC 대화 시스템
// ---------------------------------------------------------
export interface GameConversations {
	id: string;
	gameId: string;
	sessionId: string;
	branchKey: string;
	participantsKey: string;
	title: string | null;
	createdAt: number;
	updatedAt: number;
}
export interface GameMessages {
	id: string;
	seq: number;
	conversationId: string;
	role: string;
	name: string | null;
	characterId: string | null;
	content: string;
	meta: any;
	createdAt: number;
	updatedAt: number;
}

export interface NpcReply {
	text: string;
	/** 저장된 AI 메시지 id (고치기·다시 만들기) */
	messageId?: string | null;
	userMessageId?: string | null;
	events: GameEvent[];
	lines: StageLine[];
	state: RenderState;
}

// ---------------------------------------------------------
// 게임 장면들
// ---------------------------------------------------------
export interface Stage {
	background: AssetRef;
	bgm: { mood: string; asset: string };
	cg: AssetRef | null;
	standings: Record<string, CharacterStandingDetail>;
}

export interface CharacterStandingDetail {
	emotion: string;
	asset: CharacterAsset;
}

export interface AssetRef {
	id: string;
	key: string;
	path: string;
	settings: any;
}

export type Cue =
	| { type: "Bgm"; mood: string; asset: AssetRef | null }
	| { type: "Sfx"; tag: string; asset: AssetRef }
	| { type: "Voice"; asset: AssetRef }
	| { type: "Cg"; asset: AssetRef }
	| { type: "CgOff" }
	| { type: "Standing"; characterId: string; emotion: string; asset: AssetRef }
	| { type: "StandingOff"; characterId: string }
	| { type: "Motion"; characterId: string; name: string }
	| { type: "Effect"; name: string; param: string | null }
	| { type: "Background"; asset: AssetRef };

export enum LineKind {
	Narration,
	Dialogue,
	/// 텍스트 없이 연출만 있는 줄
	Direction,
}

export interface StageLine {
	kind: LineKind;
	speaker: string | null;
	text: string;
	cues: Cue[];
}

// ---------------------------------------------------------
// 게임 세이브
// 게임 세이브 저장은 GamePlayerSessions에 종속됨
// ---------------------------------------------------------
export interface GamePlayerSessions {
	id: string;
	gameId: string;
	playerId: string;
	currentSceneId: string | null;
	currentMapId: string | null;
	currentZoneId: string | null;
	currentLocationId: string | null;
	currentWeatherId: string | null;
	gameTime: number;
	flagsJson: any;
	parentSessionId: string | null;
	pendingScenesJson: any;
	inventoryJson: any;
	relationshipsJson: any;
	stageJson: any;
	forkedFromSeq: number | null;
	headSeq: number;
	updatedAt: number;
}
export interface GameJournal {
	sessionId: string;
	seq: number;
	kind: string;
	payloadJson: any | null;
	gameTime: number;
	createdAt: number;
}
export interface GameSave {
	name: string | null;
	seq: number;
	sessionId: string;
	slotNumber: number;
	currentSceneId: string | null;
	currentMapId: string | null;
	currentZoneId: string | null;
	currentLocationId: string | null;
	currentWeatherId: string | null;
	gameTime: number;
	stateSnapshot: string | null;
	updatedAt: number;
}
// ---------------------------------------------------------
// 게임 패키지 저장시 반환 데이터
// ---------------------------------------------------------
/** 삭제된 데이터와 이슈들을 모아서 반환 */
export interface SaveReport {
	deleted: Map<string, number>;
	issues: any;
}

// =========================================================
// 게임 에디터용 인터페이스
// =========================================================

export interface PackageData {
	project: GameProjectInput;
	stateDefs: GameStateDefInput[];
	maps: GameMapsInput[];
	zones: GameMapsZoneInput[];
	locations: GameLocationsInput[];
	locationConnections: GameLocationConnectionsInput[];
	interactables: GameInteractablesInput[];
	timePeriods: GameTimePeriodsInput[];
	weatherTypes: GameWeatherTypesInput[];
	weatherTransitions: GameWeatherTransitionsInput[];
	events: GameEventsInput[];
	eventConditions: GameEventConditionsInput[];
	eventCharacterPool: GameEventCharacterPoolInput[];
	encounterTables: GameEncounterTablesInput[];
	encounterEntries: GameEncounterEntriesInput[];
	scenes: GameSceneInput[];
	endings: GameEndingsInput[];
	characters: GameCharactersInput[];
	spawnLocations: GameCharacterSpawnLocationsInput[];
	items: GameItemsInput[];
	relationshipDefs: GameRelationshipDefsInput[];
	assets: GameAssetsInput[];
	audioAssets: GameAudioAssetsInput[];
	characterAssets: GameCharacterAssetsInput[];
	cgAssets: GameCgAssetsInput[];
	backgroundAssets: GameBackgroundAssetsInput[];
	worldInfoBooks: PackageWorldInfoBook[];
}

export interface GameProjectInput {
	id: string;
	title: string;
	description: string;
	templateId: string;
	apiVersion: string;
	coverImage: string | null;
	startSceneKey: string | null;
	startMapId: string | null;
	startLocationId: string | null;
	worldInfoIds: string[];
	customCss: string | null;
	settingsJson: string | null;
	createdAt: number;
	updatedAt: number;
}

export interface GameStateDefInput {
	id: string;
	gameId: string;
	description: string | null;
	key: string;
	type: string;
	defaultValue: string;
	configJson: string;
	createdAt: number;
	updatedAt: number;
}
export interface GameMapsInput {
	id: string | null;
	gameId: string;
	name: string;
	dangerLevel: number;
	defaultWeatherPoolId: string | null;
	settingsJson: unknown;
}
export interface GameMapsZoneInput {
	id: string | null;
	mapId: string;
	name: string;
	tags: string[];
	settingsJson: unknown;
}

export interface GameLocationsInput {
	id: string | null;
	zoneId: string;
	name: string;
	description: string | null;
	settingsJson: unknown;
}
export interface GameLocationConnectionsInput {
	fromId: string;
	toId: string;
	travelSeconds: number;
	condition: string | null;
	settingsJson: unknown;
}
export interface GameInteractablesInput {
	id: string | null;
	locationId: string;
	name: string;
	eventId: string | null;
}
export interface GameTimePeriodsInput {
	id: string | null;
	gameId: string;
	name: string;
	sortOrder: number;
	startMinute: number;
	endMinute: number;
}
export interface GameWeatherTypesInput {
	id: string | null;
	gameId: string;
	name: string;
	encounterRateModifier: number;
	visibilityModifier: number;
}

export interface GameWeatherTransitionsInput {
	fromWeatherId: string;
	toWeatherId: string;
	mapId: string;
	probability: number;
}
export interface GameEventsInput {
	id: string;
	gameId: string;
	eventType: string;
	triggerType: "manual" | "auto" | "interaction" | "location" | "scheduled" | "random";
	repeatPolicy: "once" | "repeatable";
	name: string;
	description: string;
	requiresNpc: boolean;
	configJson: { [key: string]: unknown };
	effectJson: { [key: string]: unknown };
	cooldownSeconds: number;
}
export interface GameEventConditionsInput {
	id: string | null;
	eventId: string;
	conditionType: string;
	conditionValue: string | null;
}
export interface GameEventCharacterPoolInput {
	id: string | null;
	eventId: string;
	gameCharacterId: string;
	spawnWeight: number;
}
export interface GameEncounterTablesInput {
	id: string | null;
	mapId: string;
	zoneId: string | null;
	locationId: string | null;
	baseTriggerChance: number;
}
export interface GameEncounterEntriesInput {
	id: string | null;
	encounterTableId: string;
	eventId: string;
	weight: number;
	timePeriodId: string | null;
	weatherId: string | null;
}
export interface GameSceneInput {
	id: string;
	gameId: string;
	sceneName: string;
	description: string | null;
	sceneKey: string;
	settingsJson: string | null;
	script?: string | null;
	choices?: GameChoiceInput[] | null;
	isEnding: boolean;
	sortOrder: number;
	createdAt: number;
	updatedAt: number;
}
export interface GameEndingsInput {
	id: string | null;
	gameId: string;
	endingKey: string;
	title: string;
	description: string;
}

export interface GameChoiceInput {
	id: string;
	sceneId: string;
	text: string;
	nextSceneId: string | null;
	condition: string | null;
	effect: string | null;
	sortOrder: number;
}

export interface GameItemsInput {
	id: string | null;
	gameId: string;
	name: string;
	description: string | null;
	icon: string | null;
	kind: string | null;
	maxStack: number;
	isUnique: boolean;
	isKey: boolean;
	consumable: boolean;
	useEffect: unknown;
	useCondition: unknown;
	onAcquireEventId: string | null;
	settingsJson: unknown;
}

export interface GameCharactersInput {
	id: string | null;
	gameId: string;
	characterId: string | null;
	name: string;
	avatar: string | null;
	description: string | null;
	firstMessage: string | null;
	exampleDialogue: string | null;
	systemPromptOverride: string | null;
	behaviorType: string | null;
	worldInfoIds: string[];
	tags: string | null;
	settingsJson: unknown;
}

export interface GameCharacterSpawnLocationsInput {
	id: string | null;
	gameCharacterId: string;
	locationId: string;
	spawnWeight: number;
	conditionJson: any;
}
export interface PackageWorldInfoBook {
	id: string;
	book: WorldInfoBookInput;
	entries: WorldInfoEntryInput[];
}

interface WorldInfoBookInput {
	id: string | null;
	name: string;
	description: string | null;
}
interface WorldInfoEntryInput {
	id: string | null;
	bookId: string;
	keys: string[];
	secondaryKeys: string[];
	content: string;
	enabled: boolean;
	priority: number;
	constant: boolean;
}

export interface GameRelationshipDefsInput {
	id: string | null;
	gameId: string;
	fromId: string;
	toId: string;
	valuesJson: any;
	tags: any;
}

export interface GameAssetsInput {
	id: string | null;
	gameId: string;
	key: string;
	name: string;
	category: string;
	kind: string;
	filePath: string;
	settingsJson: unknown | null;
}

export interface GameAudioAssetsInput extends GameAssetsInput {
	tag: string | null;
	gameCharacterId: string | null;
	transcript: string | null;
}

export interface GameCharacterAssetsInput extends GameAssetsInput {
	gameCharacterId: string | null;
	usage: "standing" | "map" | string;
	emotion: string | null;
}

export interface GameCgAssetsInput extends GameAssetsInput {
	gameCharacterId: string | null;
	description: string | null;
	prompt: string | null;
	negativePrompt: string | null;
	seed: number | null;
	generationParams: unknown | null;
	aiUsable: boolean;
}

export interface GameBackgroundAssetsInput extends GameAssetsInput {
	locationId: string | null;
	timeId: string | null;
	weatherId: string | null;
}
