use std::collections::HashMap;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSetting {
    /// 예전 "자동 업데이트" 스위치. 아무 동작에도 연결된 적이 없어 쓰지 않는다 (check_updates_on_start로 대신, 1-E-2). 저장된 값 호환용으로만 남김
    pub auto_update: bool,
    pub language: String,
    pub selected_preset: String,
    pub send_on_enter: bool,
    pub show_timestamps: bool,
    pub selected_api_profile: Option<String>,
    /// 전역 모델 (5-E-29-3, ADR-028 갱신): 자물쇠를 풀어 둔 대화가 전역 API에서 쓸 모델. None = API 프로필의 기본 모델.
    /// 전역 API를 바꾸면 화면이 비운다 (모델은 API에 딸린 이름)
    #[serde(default)]
    pub selected_model: Option<String>,
    /// 전역 추론 강도 (5-E-29-3): 자물쇠를 풀어 둔 대화가 쓴다. None·"auto" = 프리셋·공급자 기본
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    pub selected_character_id: Option<String>,
    pub selected_persona_id: String,
    // "이어하기" UX용. 실제 게임 실행 컨텍스트는 여전히 AppState.active_archive가 기준.
    pub last_played_game_id: Option<String>,
    /// phone 모드 대화에만 붙는 기본 시스템 프롬프트 ({{char}}/{{user}} 사용 가능, 비우면 붙이지 않음)
    /// - 기존 저장값에 이 필드가 없어도 설정 전체가 초기화되지 않도록 serde default 필수
    #[serde(default = "default_phone_mode_prompt")]
    pub phone_mode_prompt: String,
    /// 홈 화면 (사용자 꾸미기)
    #[serde(default)]
    pub home: HomeSetting,
    /// 앱을 켤 때 새 버전 확인 (GitHub Releases, 기본 꺼짐 — 1-E-2). 내려받기는 늘 사용자가 누른다
    #[serde(default)]
    pub check_updates_on_start: bool,
    /// 작업·사용량 상세 기록 보존 기간(일). None = 계속 보관(기본). 지난 상세는 일별 합계로 남긴다 (1-A-4)
    #[serde(default)]
    pub usage_retention_days: Option<u32>,
    /// 휴지통 보존 기간(일). None = 계속 보관. 지난 것은 휴지통을 열거나 새로 지울 때 정리 (2-A-3)
    #[serde(default = "default_trash_retention_days")]
    pub trash_retention_days: Option<u32>,
    /// 대화 CG: `이름.png`·`이름-1.png`…을 키 `이름` 하나로 묶고 보일 때 그중 하나를 무작위로 (5-E-25, 기본 꺼짐)
    #[serde(default)]
    pub cg_group_variants: bool,
    /// 응답이 끝났는데 앱이 가려져 있으면 알림 (5-E-17·18, 기본 꺼짐, 기기 전용)
    #[serde(default)]
    pub done_notify: DoneNotify,
    /// AI 호출마다 보낸 프롬프트·받은 글을 이 기기에 남김 (5-E-20-6-7, 기본 꺼짐 — 프롬프트 전체가 디스크에 남음)
    #[serde(default)]
    pub ai_call_log: bool,
    /// 응답을 기다릴 때 보이는 타이핑 표시 (5-E-17-1)
    #[serde(default)]
    pub typing: TypingIndicator,
}

/// 타이핑 표시 모양 (5-E-17-1)
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum TypingStyle {
    /// 깜빡이는 점 세 개 (예전 모양)
    #[default]
    Blink,
    /// 튀어 오르는 점 세 개
    Bounce,
    /// 점 없이 (글만, 글도 없으면 아무것도 안 보임)
    None,
    /// 사용자가 고른 그림·GIF (`image`, 없으면 깜빡이는 점) — 5-E-17-4
    Image,
}

/// 타이핑 표시 (5-E-17-1): 모양과 옆에 붙는 글
#[derive(Serialize, Deserialize, Debug, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TypingIndicator {
    pub style: TypingStyle,
    /// 점 옆의 글 (`{{char}}` = 캐릭터 이름, 비면 글 없음)
    pub text: String,
    /// 모양이 `Image`일 때 보일 그림 (앱 데이터 `themes/typing/`에 복사한 것, 5-E-17-4)
    pub image: Option<String>,
}

fn default_trash_retention_days() -> Option<u32> {
    Some(crate::service::data::trash::DEFAULT_RETENTION_DAYS)
}

/// 홈 화면: 보일 블록과 순서, 인사말, 내 글. 리소스를 쓰지 않는 화면이라 블록은 기본으로 보인다
#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct HomeSetting {
    /// 앱을 열면 홈부터 (기본 꺼짐: 지금처럼 캐릭터 화면)
    pub start_on_home: bool,
    /// 인사말 (비우면 "안녕하세요, {{user}}"). {{user}} 사용 가능
    pub greeting: String,
    /// 보일 블록 id, 위에서부터 순서대로: continue / quick / favorites / games
    pub blocks: Vec<String>,
    /// 홈 화면에만 붙는 CSS (홈이 열려 있을 때만 적용)
    pub css: String,
}

impl Default for HomeSetting {
    fn default() -> Self {
        Self {
            start_on_home: false,
            greeting: String::new(),
            blocks: ["continue", "quick", "favorites", "games"].map(String::from).to_vec(),
            css: String::new(),
        }
    }
}

pub const DEFAULT_PHONE_MODE_PROMPT: &str = "[Phone messenger chat]
{{char}} and {{user}} are apart right now, chatting through a smartphone messenger.
- {{char}} replies briefly and naturally, like real messenger messages, in the language of the conversation.
- No descriptions of actions or expressions and no narration, only the message text that would actually be sent.
- To send several messages, separate them with line breaks.";

fn default_phone_mode_prompt() -> String {
    DEFAULT_PHONE_MODE_PROMPT.to_string()
}

/// 끝나면 알림 (5-E-17·18)
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct DoneNotify {
    pub enabled: bool,
    /// 알림 제목 (비면 앱 이름, `{{char}}` = 캐릭터 이름)
    pub title: String,
    /// 알림 아이콘 그림 경로 (없으면 앱 아이콘)
    pub icon: Option<String>,
}

impl Default for AppSetting {
    fn default() -> Self {
        Self {
            auto_update: true,
            language: "ko".into(),
            selected_preset: "default".into(),
            send_on_enter: false,
            show_timestamps: true,
            selected_api_profile: None,
            selected_model: None,
            reasoning_effort: None,
            selected_character_id: None,
            selected_persona_id: "default".into(),
            last_played_game_id: None,
            phone_mode_prompt: default_phone_mode_prompt(),
            home: HomeSetting::default(),
            usage_retention_days: None,
            trash_retention_days: default_trash_retention_days(),
            check_updates_on_start: false,
            cg_group_variants: false,
            done_notify: DoneNotify::default(),
            ai_call_log: false,
            typing: TypingIndicator::default(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ColorSchema {
    Light,
    Dark,
    System,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSetting {
    pub mode: ColorSchema,
    pub select_theme: String,
    pub is_frameless: bool,
    pub accent_color: String,
    pub font_size: i64,
    /// 회색 톤 (Radix 회색 계열: slate, gray, mauve, sage, olive, sand)
    #[serde(default = "default_gray_tone")]
    pub gray_tone: String,
    /// CSS 스니펫 (옵시디언처럼 켜고 끄는 작은 CSS 조각)
    #[serde(default)]
    pub snippets: Vec<CssSnippet>,
    /// 앱 배경: 배경 목록의 파일 경로, 또는 "preset:<id>"(기본 그라데이션). 없으면 배경 없음
    #[serde(default)]
    pub background_image: Option<String>,
    /// 올려 둔 배경 이미지들 (앱 데이터 backgrounds/에 복사한 파일, GIF 가능). 테마 배경·이미지 카드가 같이 쓴다
    #[serde(default)]
    pub backgrounds: Vec<String>,
    /// 배경 위 화면의 불투명도 (0.3~1). 낮을수록 배경이 잘 보임
    #[serde(default = "default_background_opacity")]
    pub background_opacity: f32,
    /// 올려 둔 글꼴 (앱 데이터 fonts/에 복사한 파일). 앱에 글꼴을 넣어 두지 않으므로 목록은 사용자가 올린 것뿐
    #[serde(default)]
    pub fonts: Vec<UserFont>,
    /// 앱 글꼴: `fonts` 중 하나의 파일 경로. 없으면 기본 글꼴
    #[serde(default)]
    pub app_font: Option<String>,
}

/// 올린 글꼴 한 개
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UserFont {
    /// 앱 데이터 fonts/ 안의 파일 경로
    pub path: String,
    /// 목록에 보일 이름 (올릴 때의 파일 이름)
    pub name: String,
}

fn default_background_opacity() -> f32 {
    0.82
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CssSnippet {
    pub id: String,
    pub name: String,
    pub css: String,
    pub enabled: bool,
}

fn default_gray_tone() -> String {
    "slate".into()
}

impl Default for ThemeSetting {
    fn default() -> Self {
        Self {
            mode: ColorSchema::System,
            select_theme: "default".into(),
            is_frameless: true,
            accent_color: "#8b85f9".into(),
            font_size: 16,
            gray_tone: default_gray_tone(),
            snippets: vec![],
            background_image: None,
            backgrounds: vec![],
            background_opacity: default_background_opacity(),
            fonts: vec![],
            app_font: None,
        }
    }
}

/// 테마 색: 화면 모드별로 덮어쓸 CSS 변수 ("--bg" → "#ffffff"). 비어 있으면 앱 기본값
#[derive(Serialize, Deserialize, Debug, Clone, Default, specta::Type)]
pub struct ThemeColors {
    #[serde(default)]
    pub light: HashMap<String, String>,
    #[serde(default)]
    pub dark: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CssThemeMeta {
    #[serde(default, skip_deserializing, skip_serializing_if = "String::is_empty")]
    pub filename: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
    /// 목록 카드 미리보기용
    #[serde(default)]
    pub colors: ThemeColors,
}

// 2. 기존 저장/Tauri 통신용 전체 구조체
#[derive(Serialize, Deserialize, Debug, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CssTheme {
    #[serde(default, skip_deserializing, skip_serializing_if = "String::is_empty")]
    pub filename: String,
    pub name: String,
    pub author: String,
    pub version: String,
    /// 직접 쓰는 CSS (색 편집으로 부족할 때)
    #[serde(default)]
    pub css_content: String,
    #[serde(default)]
    pub colors: ThemeColors,
    /// 이 테마를 고르면 쓸 강조색 (팔레트 id 또는 #rrggbb). 없으면 설정을 따름 (5-E-14)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    /// 이 테마를 고르면 쓸 회색 톤. 없으면 설정을 따름
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gray_tone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct FeatureSetting {
    pub feature: HashMap<String, bool>,
}

impl FeatureSetting {
    /// 위젯 창을 열어 둘지: 위젯이 켜져 있고 하위 기능(widget.*)이 하나라도 켜져 있을 때.
    /// 하위 기능 값이 하나도 없으면(처음 켬) 위젯 창이 기본값을 등록하도록 연다
    pub fn widget_window_wanted(&self) -> bool {
        if !self.feature.get("widget").copied().unwrap_or(false) {
            return false;
        }
        let mut subs = self.feature.iter().filter(|(k, _)| k.starts_with("widget.")).peekable();
        subs.peek().is_none() || subs.any(|(_, on)| *on)
    }
}

impl Default for FeatureSetting {
    fn default() -> Self {
        Self { feature: HashMap::new() }
    }
}

// ---------------------------------------------------------
// 코어 플러그인 설정 (번역 · 음성 · 이미지 생성). 켜짐/꺼짐은 FeatureSetting, 세부 설정은 여기.
// ---------------------------------------------------------
#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PluginSetting {
    pub translate: TranslateSetting,
    pub tts: TtsSetting,
    pub image_gen: ImageGenSetting,
    pub phone: PhoneSetting,
    /// 앱 프롬프트 (사용자가 고친 것만, 비우면 기본값 — service::app_prompts)
    #[serde(default)]
    pub prompts: AppPrompts,
    /// 폴더 동기화 (2-C) — 경로가 기기마다 달라 기기 전용 (settings_scope)
    #[serde(default)]
    pub sync: SyncSetting,
    /// 메시지 linter 규칙 (3-B, ADR-014). 켜짐/꺼짐은 코어 플러그인 `lint`(기본 꺼짐)
    #[serde(default)]
    pub lint: LintSetting,
    /// 메모리 임베딩 검색 (3-C, ADR-015) — 기기 전용
    #[serde(default)]
    pub memory: MemorySetting,
    /// 메모리 확장: 대화 요약 (3-D, ADR-016) — 기기 전용
    #[serde(default)]
    pub summary: SummarySetting,
    /// 자동 재생성 (3-B-4, ADR-017) — 기기 전용
    #[serde(default)]
    pub rewrite: RewriteSetting,
    /// 대필 (5-D, ADR-018) — 기기 전용
    #[serde(default)]
    pub ghost: GhostSetting,
    /// 빠른 답장 세트 (5-E-20-5-3, ADR-025) — 같이 씀. 켜짐/꺼짐은 코어 플러그인 `slash`
    #[serde(default)]
    pub quick_reply: QuickReplySetting,
    /// 상태창·변수 (5-F-4, ADR-029) — 기기 전용
    #[serde(default)]
    pub status: StatusSetting,
}

/// 상태창·변수 플러그인 설정
#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct StatusSetting {
    /// AI가 대화 변수를 읽고 바꾸기 (5-F-4-4, 기본 꺼짐). 켜면 변수 규칙과 지금 값이 프롬프트에 들어가 응답마다 토큰이 는다
    pub ai_vars: bool,
}

/// 빠른 답장: 입력칸 위 버튼. 누르면 `/`로 시작하는 글은 스크립트로 실행, 아니면 보내거나 입력칸에 넣음
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct QuickReplySetting {
    /// 예전(5-E-20-5-3) 자리. 지금은 DB(quick_reply_sets)에 두고, 처음 목록을 읽을 때 옮긴 뒤 비운다
    pub sets: Vec<LegacyQuickReplySet>,
    /// 슬래시 스크립트 한 번 실행에서 AI를 부를 수 있는 횟수 (1~50, ADR-025 5-E-20-6-0)
    pub ai_limit: u32,
    /// 전역 변수 (/setglobalvar, 기기 사이에 같이 씀)
    pub globals: std::collections::BTreeMap<String, String>,
}

impl Default for QuickReplySetting {
    fn default() -> Self {
        Self {
            sets: vec![],
            ai_limit: 5,
            globals: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct LegacyQuickReplySet {
    pub id: String,
    pub name: String,
    /// 대화에서 고르지 않았을 때 모든 대화에 보임
    pub enabled: bool,
    /// 스크립트가 아닌 글: true면 바로 보냄(AI 답장 요금), false면 입력칸에 넣기만
    pub auto_send: bool,
    pub items: Vec<QuickReply>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct QuickReply {
    pub id: String,
    pub label: String,
    /// 보낼 글 또는 슬래시 스크립트
    pub message: String,
    /// 버튼 줄에 보이지 않음 (스크립트에서만 쓰는 것)
    pub hidden: bool,
}

/// 대필 설정. 켜짐/꺼짐은 코어 플러그인 `ghostwrite`(기본 꺼짐)
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct GhostSetting {
    /// 대필에 쓸 API (비우면 그 대화의 API)
    pub api_profile_id: Option<String>,
    /// 대필에 쓸 모델 (비우면 그 API의 기본 모델. 대필 API를 따로 정했을 때만 쓴다, ADR-020)
    pub model: String,
    /// short / medium / long
    pub length: String,
    /// 추가 지침 한 줄 (300자까지)
    pub guide: String,
    /// 펜 버튼을 누르면 초안 작성 없이 바로 AI에게 요청 (한 번 누르면 한 번의 요금)
    pub send_without_draft: bool,
}

impl Default for GhostSetting {
    fn default() -> Self {
        Self {
            api_profile_id: None,
            model: String::new(),
            length: "medium".into(),
            guide: String::new(),
            send_without_draft: false,
        }
    }
}

/// 자동 재생성 설정. 켜짐/꺼짐은 코어 플러그인 `rewrite`(기본 꺼짐, 켜는 것이 동의). 꺼져 있으면 linter가 쓸 수 없다
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct RewriteSetting {
    /// 한 응답에 자동으로 더 만드는 최대 횟수 (기본 1, 안전상 3까지) — 한 번마다 응답 하나의 요금
    pub max_attempts: u32,
    /// 아래 금지어가 응답에 나오면 다시 쓰기. 메시지 linter의 금지어 목록과는 따로다(linter는 표시·지우기, 여기는 다시 만들기)
    pub banned: bool,
    /// 자동 재생성의 금지어 목록 (대소문자 구분 없음, 100개까지)
    pub banned_words: Vec<String>,
    /// 단어 반복 오류가 걸리면 다시 쓰기
    pub stuck: bool,
}

impl Default for RewriteSetting {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            banned: true,
            banned_words: vec![],
            stuck: true,
        }
    }
}

/// 대화 요약 설정. 켜짐/꺼짐은 코어 플러그인 `summary`(기본 꺼짐, 켜는 것이 동의)
#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SummarySetting {
    /// 요약에 쓸 API 프로필 (비우면 그 대화가 쓰는 API)
    pub api_profile_id: Option<String>,
    /// 요약에 쓸 모델 (비우면 그 API의 기본 모델. 요약 API를 따로 정했을 때만 쓴다, ADR-020)
    pub model: String,
}

/// 메모리 임베딩 검색 설정. API 프로필·모델은 기기마다 다르므로 통째로 기기 전용
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct MemorySetting {
    /// 임베딩에 쓸 API 프로필 (비우면 색인·검색을 할 수 없음 — 앱이 고른 채팅 API를 몰래 쓰지 않는다)
    pub api_profile_id: Option<String>,
    /// 임베딩 모델 이름 (사용자가 입력. 앱은 모델·가격을 내장하지 않음)
    pub model: String,
    /// 줄일 차원 (지원하는 모델만). 비우면 모델 기본
    pub dimensions: Option<u32>,
    /// 한 번 색인에서 임베딩할 최대 토큰(추정). 넘으면 한 번 더 확인
    pub token_cap: u32,
    /// 문서·질의 앞에 붙일 글 (e5의 "passage: "/"query: " 같은 모델용)
    pub doc_prefix: String,
    pub query_prefix: String,
    /// 보낼 때 관련된 지난 대화를 프롬프트에 넣기 (3-C-5). 보낼 때마다 임베딩 1회가 자동 실행으로 듦 — 기본 꺼짐
    pub auto_recall: bool,
    /// 넣는 글의 토큰 상한
    pub recall_tokens: u32,
    /// 최대 chunk 수
    pub recall_top_k: u32,
    /// 어디서 찾나: "conversation"(이 대화) / "character"(같은 캐릭터의 대화) / "all"(전체)
    pub recall_scope: String,
}

impl Default for MemorySetting {
    fn default() -> Self {
        Self {
            api_profile_id: None,
            model: String::new(),
            dimensions: None,
            token_cap: 200_000,
            doc_prefix: String::new(),
            query_prefix: String::new(),
            auto_recall: false,
            recall_tokens: 1500,
            recall_top_k: 4,
            recall_scope: "character".into(),
        }
    }
}

/// linter 규칙 하나의 동작
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum LintMode {
    Off,
    /// 표시만: 글은 그대로 두고 메시지 아래에 알림
    #[default]
    Report,
    /// 자동 고침: 고치고 원문을 meta에 남김 (되돌리기 가능)
    Fix,
}

/// 메시지 linter 규칙 (요금 없이 앱 안에서 도는 것만 — 3-B-2). 글자 수 규칙은 표시만 할 수 있다
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct LintSetting {
    /// 닫히지 않은 태그·따옴표·마크다운(코드 펜스·`**`·`*`)
    pub unclosed: LintMode,
    /// 같은 문단 반복
    pub repeat: LintMode,
    /// 단어 반복 오류: 같은 말을 끝없이 되풀이하는 고장 (고치면 두 번까지만 남김)
    pub stuck: LintMode,
    /// 내 대사를 AI가 대신 쓴 줄(`이름: …`)
    pub impersonate: LintMode,
    /// 금지어
    pub banned: LintMode,
    pub banned_words: Vec<String>,
    /// 글자 수 범위 (표시만)
    pub length: LintMode,
    /// 0 = 제한 없음
    pub min_chars: u32,
    pub max_chars: u32,
}

impl Default for LintSetting {
    fn default() -> Self {
        Self {
            unclosed: LintMode::Report,
            repeat: LintMode::Report,
            stuck: LintMode::Report,
            impersonate: LintMode::Report,
            banned: LintMode::Off,
            banned_words: vec![],
            length: LintMode::Off,
            min_chars: 0,
            max_chars: 0,
        }
    }
}

/// 폴더 동기화 설정 (CLAUDE.md 2-C, ADR-011)
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncSetting {
    /// 동기화 폴더 (OneDrive·Dropbox처럼 이미 쓰는 동기화 앱의 폴더 안). 없으면 폴더 동기화를 하지 않음
    pub folder: Option<String>,
    /// 자동 동기화 (기본 꺼짐): 앱을 켤 때 한 번, 그 뒤 interval_minutes마다
    pub auto: bool,
    pub interval_minutes: u32,
    /// 지금 동기화·자동 동기화가 쓸 저장소: "folder" / "drive" (2-D)
    pub target: String,
    /// 연결한 구글 계정 (화면 표시용. 토큰은 keyring에만)
    pub drive_email: Option<String>,
    /// Android: Wi-Fi(요금이 들지 않는 연결)에서만 자동 동기화, 직접 누를 때는 먼저 묻기 (기본 켜짐)
    pub wifi_only: bool,
}

impl Default for SyncSetting {
    fn default() -> Self {
        Self {
            folder: None,
            auto: false,
            interval_minutes: 30,
            target: "folder".into(),
            drive_email: None,
            wifi_only: true,
        }
    }
}

/// 앱이 AI에 보내는 고정 지시문. 자리마다 사용자가 여러 개 만들어 두고 하나를 고른다 (고르지 않으면 기본값)
#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AppPrompts {
    /// 번역 지시 ({{lang}} = 옮길 언어)
    pub translate: PromptSlot,
    /// 장면 → 그림 프롬프트 쓰기
    pub image_prompt: PromptSlot,
    /// 장면 → NovelAI 태그 쓰기
    pub image_prompt_nai: PromptSlot,
    /// 대화의 캐릭터 이미지(<cg>) 규칙 ({{img_keys}} = 키 목록 — 예전 {{keys}}도 됨, {{char}} = 캐릭터)
    pub chat_images: PromptSlot,
    /// 이미지 자동 생성: 글쓰기 AI에게 응답 끝 <image> 그림 프롬프트를 쓰게 하는 지시
    pub auto_image: PromptSlot,
    /// 게임 대화 규칙 (한 줄에 하나)
    pub game_rules: PromptSlot,
    /// 씬 입력(서술자) 규칙 (한 줄에 하나)
    pub scene_rules: PromptSlot,
    /// 대화 요약 지시 ({max} = 요약 최대 토큰)
    pub summary: PromptSlot,
}

/// 사용자가 만든 프롬프트 하나
#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptVariant {
    pub id: String,
    pub name: String,
    pub text: String,
}

/// 프롬프트 자리: 만든 것들과 고른 것 (selected가 비었거나 없는 id면 기본값)
#[derive(Debug, Serialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptSlot {
    pub selected: String,
    pub items: Vec<PromptVariant>,
}

impl PromptSlot {
    /// 고른 프롬프트의 글 (기본값을 쓰면 None)
    pub fn chosen(&self) -> Option<&str> {
        if self.selected.is_empty() {
            return None;
        }
        self.items
            .iter()
            .find(|v| v.id == self.selected)
            .map(|v| v.text.as_str())
            .filter(|t| !t.trim().is_empty())
    }
}

/// 예전(한 줄 글) 저장값도 읽는다: 글이 있으면 "내 프롬프트" 하나를 만들어 고른 상태로
impl<'de> Deserialize<'de> for PromptSlot {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Text(String),
            Slot {
                #[serde(default)]
                selected: String,
                #[serde(default)]
                items: Vec<PromptVariant>,
            },
        }
        Ok(match Raw::deserialize(d)? {
            Raw::Text(t) if t.trim().is_empty() => PromptSlot::default(),
            Raw::Text(t) => PromptSlot {
                selected: "mine".into(),
                items: vec![PromptVariant {
                    id: "mine".into(),
                    name: "내 프롬프트".into(),
                    text: t,
                }],
            },
            Raw::Slot { selected, items } => PromptSlot { selected, items },
        })
    }
}

/// 폰 대화의 기본 API·프리셋 (대화에 따로 고르지 않았을 때. 비우면 앱 기본값)
#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PhoneSetting {
    pub api_profile_id: Option<String>,
    /// 폰 대화의 기본 모델 (비우면 그 API의 기본 모델. ADR-020). API를 정했을 때만 쓴다
    pub model: String,
    pub preset_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TranslateSetting {
    /// 번역에 쓸 API 프로필 (비우면 앱에서 고른 API)
    pub api_profile_id: Option<String>,
    /// 번역에 쓸 모델 (비우면 그 API의 기본 모델. ADR-020)
    pub model: String,
    /// AI 답변을 이 언어로 (예: "한국어")
    pub target_lang: String,
    /// 내 입력을 이 언어로 (예: "English")
    pub input_lang: String,
    /// 새 AI 답변을 자동으로 번역
    pub auto_translate: bool,
}

impl Default for TranslateSetting {
    fn default() -> Self {
        Self {
            api_profile_id: None,
            model: String::new(),
            target_lang: "한국어".into(),
            input_lang: "English".into(),
            auto_translate: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TtsSetting {
    /// 음성 API 프로필 (주소 + 키). 종류가 ElevenLabs면 ElevenLabs, 아니면 OpenAI 호환 /audio/speech
    pub api_profile_id: Option<String>,
    /// OpenAI: tts-1 등 / ElevenLabs: model_id (eleven_multilingual_v2 등)
    pub model: String,
    /// OpenAI: 목소리 이름 (alloy 등) / ElevenLabs: voice_id
    pub voice: String,
    /// 0.25 ~ 4.0
    pub speed: f32,
    /// 새 AI 답변을 자동으로 읽기
    pub auto_read: bool,
    /// *지문*은 빼고 대사만 읽기
    pub dialogue_only: bool,
}

impl Default for TtsSetting {
    fn default() -> Self {
        Self {
            api_profile_id: None,
            model: "eleven_multilingual_v2".into(),
            voice: String::new(),
            speed: 1.0,
            auto_read: false,
            dialogue_only: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ImageGenSetting {
    /// 이미지 API 프로필 (주소 + 키). 종류가 NovelAI면 NovelAI, 아니면 OpenAI 호환 /images/generations
    pub api_profile_id: Option<String>,
    pub model: String,
    /// 예: "1024x1024", NovelAI는 "832x1216"
    pub size: String,
    /// 모든 그림에 붙일 화풍·품질 태그 (예: "anime style, soft lighting" / NovelAI: "masterpiece, best quality")
    pub style_prompt: String,
    /// 메시지를 바로 쓰지 않고 채팅 AI가 먼저 이미지 프롬프트를 쓴다
    pub ai_prompt: bool,
    /// 이미지 프롬프트를 쓸 채팅 API (비우면 대화/앱의 것)
    pub prompt_api_profile_id: Option<String>,
    /// 이미지 프롬프트를 쓸 채팅 모델 (비우면 그 API의 기본 모델. ADR-020)
    pub prompt_model: String,
    /// NovelAI 생성 프리셋 (OpenAI는 쓰지 않음)
    pub negative_prompt: String,
    pub steps: u32,
    pub scale: f32,
    pub sampler: String,
    /// NovelAI 품질 태그: "standard" / "light" / "none" (프롬프트 끝에 붙임)
    #[serde(default = "default_quality")]
    pub quality: String,
}

fn default_quality() -> String {
    "standard".into()
}

impl Default for ImageGenSetting {
    fn default() -> Self {
        Self {
            api_profile_id: None,
            model: "nai-diffusion-4-5-full".into(),
            size: "832x1216".into(),
            style_prompt: String::new(),
            ai_prompt: true,
            prompt_api_profile_id: None,
            prompt_model: String::new(),
            negative_prompt: "lowres, bad anatomy, bad hands, text, error, missing fingers, worst quality, low quality, jpeg artifacts, watermark, blurry"
                .into(),
            steps: 28,
            scale: 5.0,
            sampler: "k_euler_ancestral".into(),
            quality: default_quality(),
        }
    }
}

// 각 설정 타입이 자기 "키"와 "이벤트 이름"만 정의하면 됨
pub trait SettingsDomain: Serialize + DeserializeOwned + Default + Clone + Send + Sync + 'static {
    const STORE_KEY: &'static str;
    const EVENT_NAME: &'static str;
}

impl SettingsDomain for AppSetting {
    const STORE_KEY: &'static str = "app-setting";
    const EVENT_NAME: &'static str = "app_setting::changed";
}

impl SettingsDomain for ThemeSetting {
    const STORE_KEY: &'static str = "theme-setting";
    const EVENT_NAME: &'static str = "theme_setting::changed";
}
impl SettingsDomain for FeatureSetting {
    const STORE_KEY: &'static str = "feature-setting";
    const EVENT_NAME: &'static str = "feature_setting::changed";
}
impl SettingsDomain for PluginSetting {
    const STORE_KEY: &'static str = "plugin-setting";
    const EVENT_NAME: &'static str = "plugin_setting::changed";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_app_setting_without_phone_prompt_keeps_other_fields() {
        // phone_mode_prompt가 생기기 전에 저장된 설정
        let old = serde_json::json!({
            "autoUpdate": false,
            "language": "en",
            "selectedPreset": "p1",
            "sendOnEnter": true,
            "showTimestamps": false,
            "selectedApiProfile": "api-1",
            "selectedCharacterId": null,
            "selectedPersonaId": "me",
            "lastPlayedGameId": null
        });
        let s: AppSetting = serde_json::from_value(old).expect("기존 설정이 그대로 읽혀야 함");
        assert_eq!(s.language, "en");
        assert_eq!(s.selected_api_profile.as_deref(), Some("api-1"));
        assert_eq!(s.phone_mode_prompt, DEFAULT_PHONE_MODE_PROMPT);
        // 타이핑 표시가 없던 설정 → 예전 모양 (5-E-17-1)
        assert_eq!(s.typing.style, TypingStyle::Blink);
        assert_eq!(s.typing.text, "");
    }

    #[test]
    fn typing_indicator_reads_partial_values() {
        let t: TypingIndicator = serde_json::from_value(serde_json::json!({ "style": "bounce" })).unwrap();
        assert_eq!(t.style, TypingStyle::Bounce);
        assert_eq!(t.text, "");
        let t: TypingIndicator = serde_json::from_value(serde_json::json!({ "style": "none", "text": "{{char}} 입력 중…" })).unwrap();
        assert_eq!(t.style, TypingStyle::None);
        assert_eq!(serde_json::to_value(t).unwrap()["style"], "none");
    }
}

#[cfg(test)]
mod feature_tests {
    use super::FeatureSetting;

    fn fs(pairs: &[(&str, bool)]) -> FeatureSetting {
        FeatureSetting {
            feature: pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        }
    }

    #[test]
    fn widget_window_follows_widget_and_sub_features() {
        assert!(!fs(&[]).widget_window_wanted());
        assert!(fs(&[("widget", true)]).widget_window_wanted(), "처음 켜면 기본값 등록을 위해 연다");
        assert!(!fs(&[("widget", true), ("widget.music", false), ("widget.mascot", false)]).widget_window_wanted());
        assert!(fs(&[("widget", true), ("widget.music", true), ("widget.mascot", false)]).widget_window_wanted());
        assert!(!fs(&[("widget", false), ("widget.music", true)]).widget_window_wanted());
    }
}
