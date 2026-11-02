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
	settingJson: string;
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
	view: any;
	issues: Issue[];
}

export interface Issue {
	severity: Severity;
	message: string;
}

export enum Severity {
	Error,
	Warning,
}

// ---------------------------------------------------------
// // 런타임 시 게임 엔진에서 넘어오는 데이터
// ---------------------------------------------------------
/**
 * 랜더링 데이터
 * 정적 데이터에서 변한 데이터만 반환된다.
 */
export interface GameResponse {
	// 게임엔진에서 돌아간 이벤트 목록
	// 무시해도 됨
	events: GameEvent[];
	// 화면 표시 연출용
	state: RenderState;
}

export interface RenderState {
	sessionId: string;
	locationId: string | null;
	mapId: string;
	zoneId: string | null;
	gameTime: number;
	weatherID: string;
	timePeriodId: string;
	currentSceneId: string | null;
	activeNpcId: string | null;
	availableChoices: string[];
	flags: any;
	inventory: any;
	relationships: any;
	stage: Stage;
	presentNpcs: string[];
}

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
	events: GameEvent[];
	state: RenderState;
}

// ---------------------------------------------------------
// 게임 장면들
// ---------------------------------------------------------
export interface Stage {
	background: AssetRef;
	bgm: { mood: string; asset: string };
	cg: AssetRef | null;
	standings: { 캐릭터id: { emotion: string; asset: string } };
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
	forkedFromSeq: number | null;
	headSeq: number;
	updatedAt: number;
}
export interface GameJournal {
	sessionId: string;
	seq: number;
	kind: string;
	payloadJson: any;
	gameTime: number;
	createdAt: number;
}
export interface GameSave {
	id: string;
	gameId: string;
	sessionId: string;
	slotNumber: number;
	currentSceneId: string | null;
	currentMapId: string | null;
	currentZoneId: string | null;
	currentLocationId: string | null;
	gameTime: number;
	currentWeatherId: string | null;
	stateSnapshot: string | null;
	updatedAt: number;
}

// ---------------------------------------------------------
// 게임 커맨드들
// ---------------------------------------------------------
export enum GameCommand {
	MoveTo, // 이동
	Interact, // 상호작용
	Talk, // NPC 대화
	StopTalk, // 대화 멈춤
	SetVariable, // 변수 설정
	Choose, // 분기 노드
	AdvanceTime, // 시간 흐름
	UseItem, // 아이템 사용
	DropItem, // 아이템 버리기
}

export enum GameEvent {
	GameStarted, // 게임 시작
	LocationChanged, // 이동
	ZoneChanged, // zone 이동 (맵 순서: map>zone>location)
	TimeAdvanceRequested, // 시간 흘렀을 경우 뭐가 바뀌었는지 확인하기 위한 이벤트
	TimeAdvanced, // 시간 변경
	WeatherChanged, // 날씨 변경
	TimePeriodChanged, // 시간대 변경
	NpcMoved, // NPC 이동
	EventTriggered, // 이벤트 트리거
	SceneRequested, // 씬 요청
	EventRequested, // 이벤트 요청
	TriggerActivated, // 트리거 활성화
	DialogueStarted, // 다일로그 시작
	DialogueEnded, // 다일로그 끝
	ChoicePresented, // 선택지
	ChoiceMade,
	SceneChanged,
	SceneExited,
	VariableChanged,
	ItemChangeRequested,
	ItemUsed,
	RelationChangeRequested,
	RelationChanged,
	ItemAdded,
	ItemRemoved,
	GameEnded,
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
	worldInfoBooks: PackageWorldInfoBook[];
	assets: GameAssetsInput[];
}

export interface GameBundleRow {
	game: GameProject;
	stateDefs: GameStateDef[];
	scenes: GameScene[];
	events: GameEvents[];
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
	settingJson: string;
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

export interface GameStateDef {
	id: string;
	gameId: string;
	description: string | null;
	key: string;
	type: string;
	defaultValue: string;
	createdAt: number;
	updatedAt: number;
}

export interface GameChoice {
	id: string;
	sceneId: string;
	text: string;
	nextSceneId: string | null;
	effect: string | null;
	sortOrder: number;
}

export interface GameMaps {
	id: string;
	gameId: string;
	name: string;
	dangerLevel: number;
	defaultWeatherPoolId: string | null;
	settingsJson: unknown;
	createdAt: number;
	updatedAt: number;
}

export interface GameMapsZone {
	id: string;
	mapId: string;
	name: string;
	tags: string[];
	settingsJson: unknown;
	createdAt: number;
	updatedAt: number;
}

export interface GameLocations {
	id: string;
	zoneId: string;
	name: string;
	description: string | null;
	settingsJson: unknown;
}
export interface GameLocationConnections {
	fromId: string;
	toId: string;
	travelSeconds: number;
	condition: string | null;
	settingsJson: unknown;
}

export interface GameInteractables {
	id: string;
	locationId: string;
	name: string;
	eventId: string | null;
}

export interface GameTimePeriods {
	id: string;
	gameId: string;
	name: string;
	sortOrder: number;
	startMinute: number;
	endMinute: number;
}

export interface GameWeatherTypes {
	id: string;
	gameId: string;
	name: string;
	encounterRateModifier: number;
	visibilityModifier: number;
	createdAt: number;
	updatedAt: number;
}

export interface GameWeatherTransitions {
	fromWeatherId: string;
	toWeatherId: string;
	mapId: string;
	probability: number;
}

export interface GameEvents {
	id: string;
	gameId: string;
	eventType: string;
	name: string;
	description: string;
	requiresNpc: boolean;
	configJson: { [key: string]: unknown };
	effectJson: { [key: string]: unknown };
	cooldownSeconds: number;
	createdAt: number;
	updatedAt: number;
}

export interface GameEventConditions {
	id: string;
	eventId: string;
	conditionType: string;
	conditionValue: string | null;
}
export interface GameEventCharacterPool {
	eventId: string;
	gameCharacterId: string;
	spawnWeight: number;
}

export interface GameEncounterTables {
	id: string;
	mapId: string;
	zoneId: string | null;
	locationId: string | null;
	baseTriggerChance: number;
}

export interface GameEncounterEntries {
	id: string;
	encounterTableId: string;
	eventId: string;
	weight: number;
	timePeriodId: string | null;
	weatherId: string | null;
}

export interface GameScene {
	id: string;
	gameId: string;
	sceneName: string;
	description: string | null;
	sceneKey: string;
	settingsJson: string | null;
	isEnding: boolean;
	sortOrder: number;
	createdAt: number;
	updatedAt: number;
}

export interface GameEndings {
	id: string;
	gameId: string;
	endingKey: string;
	title: string;
	description: string;
}
export interface GameCharacters {
	id: string;
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
	createdAt: number;
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
