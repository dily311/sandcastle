//! 백엔드 오류 다국어 키·인자 분류기 (ADR-032 ④, roadmap 5-C-2-9).
//!
//! - `AppError::to_response()`와 `TaskError::to_info()`가 한국어 원문 `message`는 로그·기존 시험 호환을 위해 그대로 두면서,
//!   프런트엔드가 현재 언어(`ko`/`en`/…)로 정확히 치환할 수 있도록 `key`(`errors.detail.<id>` 또는 `errors.gameBlocked.<reason>`)와
//!   이름 붙은 `args`(`BTreeMap<String, String>`)를 함께 채운다.
//! - `플러그인 manifest.json: …`·`플러그인 묶음: …`처럼 안쪽 사유를 감싸는 메시지는 안쪽 사유도 재귀 분류해
//!   `subKey`로 함께 실어 보낸다.

use serde_json::Value;
use std::collections::BTreeMap;

use crate::error::ErrorCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorI18n {
    pub key: String,
    pub args: Option<BTreeMap<String, String>>,
}

struct TemplateRule {
    key: &'static str,
    segs: &'static [&'static str],
    args: &'static [&'static str],
}

const EXACT_RULES: &[(&str, &str)] = &[
    ("저장소 보관본의 모양이 올바르지 않음", "errors.detail.storageInvalidShape"),
    ("author·description이 너무 김", "errors.detail.manifestAuthorDescTooLong"),
    ("apiVersion이 없음", "errors.detail.manifestMissingApiVersion"),
    ("panels를 쓰려면 'ui.panel' 권한이 필요함", "errors.detail.manifestPanelsNeedPerm"),
    ("commands를 쓰려면 'ui.command' 권한이 필요함", "errors.detail.manifestCommandsNeedPerm"),
    ("폴더 밖의 경로", "errors.detail.pkgPathOutsideFolder"),
    ("맨 위에 manifest.json이 없음", "errors.detail.pkgMissingRootManifest"),
    ("커뮤니티 플러그인이 꺼져 있음", "errors.detail.communityDisabled"),
    (
        "미리보기 뒤에 플러그인 파일이 바뀌었음 — 다시 열어 확인하세요",
        "errors.detail.communityChangedSincePreview",
    ),
    (
        "지금 쓰는 판은 지울 수 없음 — 다른 판을 고른 뒤에 지우세요",
        "errors.detail.communityCannotDeleteActiveVer",
    ),
    ("플러그인이 요청한 권한 전부에 동의해야 켤 수 있음", "errors.detail.communityConsentRequired"),
    ("플러그인 폴더를 읽을 수 없음", "errors.detail.communityCannotReadFolder"),
    (
        "ai.generate는 call_user_plugin 커맨드로만 부를 수 있음",
        "errors.detail.communityAiGenerateRoute",
    ),
    ("prompt가 필요함 (글)", "errors.detail.communityAiNeedsPrompt"),
    ("system은 글이어야 함", "errors.detail.communityAiSystemString"),
    ("maxTokens는 양의 정수", "errors.detail.communityAiMaxTokensInt"),
    ("temperature는 숫자", "errors.detail.communityAiTempNumber"),
    (
        "이 플러그인은 사용자가 누른 명령 밖에서 AI를 부를 수 없음 — 설정 > 커뮤니티 플러그인에서 '자동 AI 호출'을 켜면 가능 (요금이 들 수 있음)",
        "errors.detail.communityAiAutoOff",
    ),
    ("이전 AI 호출이 끝나지 않았음 — 하나씩 부르세요", "errors.detail.communityAiBusy"),
    ("메시지 meta를 읽을 수 없어 후처리를 저장하지 않음", "errors.detail.postprocessMetaReadErr"),
    ("후처리한 글이 너무 김", "errors.detail.postprocessTextTooLong"),
    ("글을 바꾼 플러그인이 없는데 글이 다름", "errors.detail.postprocessChangedWithoutEditor"),
    (
        "플러그인이 응답에 HTML 코드 블록을 새로 넣거나 바꿀 수 없음 (앱이 그대로 그려 화면에서 실행될 수 있어서)",
        "errors.detail.postprocessNoHtmlCodeBlock",
    ),
    ("AI 응답만 후처리함", "errors.detail.postprocessAssistantOnly"),
    ("후처리하는 사이 메시지가 바뀌어 저장하지 않음", "errors.detail.postprocessMsgChangedDuring"),
    ("플러그인이 고친 메시지가 아님", "errors.detail.postprocessNotEditedByPlugin"),
    ("플러그인이 고친 뒤 메시지를 또 고쳐서 되돌리지 않음", "errors.detail.postprocessEditedAgain"),
    ("바꿀 칸이 없음", "errors.detail.proposalNoFields"),
    ("고칠 항목 id가 필요함", "errors.detail.proposalTargetIdRequired"),
    ("지금과 달라지는 칸이 없음", "errors.detail.proposalNoDiff"),
    ("이 종류는 새로 만들기를 제안할 수 없음", "errors.detail.proposalCreateUnsupported"),
    ("넣을 곳(parentId)이 필요함", "errors.detail.proposalParentIdRequired"),
    ("고칠 항목이 지워졌음 — 거절하세요", "errors.detail.proposalTargetDeleted"),
    (
        "제안한 뒤 원본이 바뀌어 적용하지 않음 — 거절하고 다시 제안받으세요",
        "errors.detail.proposalStaleSource",
    ),
    ("넣을 곳이 지워졌음 — 거절하세요", "errors.detail.proposalParentDeleted"),
    ("적용한 제안만 되돌릴 수 있음", "errors.detail.proposalOnlyAppliedRevertible"),
    ("적용한 뒤 원본이 또 바뀌어 되돌리지 않음", "errors.detail.proposalEditedAfterApply"),
    ("AI 서버 응답 시간이 초과되었습니다", "errors.detail.aiTimeout"),
    (
        "도구와 구조화 출력은 한 요청에 같이 쓸 수 없음 (공급자마다 조합 지원이 다름)",
        "errors.detail.toolsAndStructuredConflict",
    ),
    ("도구가 없는데 도구를 쓰게 할 수 없음", "errors.detail.toolsChoiceWithoutTools"),
    ("도구 호출 기록(tail)에는 도구 목록도 필요함", "errors.detail.toolsTailNeedsDefs"),
    ("앞선 도구 호출에 결과가 없음", "errors.detail.toolsMissingResult"),
    (
        "도구 결과가 직전 호출들과 짝이 맞지 않음 (id가 다르거나 빠짐)",
        "errors.detail.toolsResultMismatch",
    ),
    ("선택된 API 프로필이 없습니다", "errors.detail.noSelectedApiProfile"),
    ("임베딩 모델 이름이 비어 있음", "errors.detail.embedModelEmpty"),
    ("임베딩할 글이 없음", "errors.detail.embedInputEmpty"),
    ("벡터 길이는 1 이상이어야 함", "errors.detail.embedDimPositive"),
    ("AI 요청이 취소되었습니다", "errors.detail.aiRequestCancelled"),
    ("API를 먼저 고르세요 (대화 설정 또는 설정 > API 키)", "errors.detail.selectApiChatHint"),
    ("API를 먼저 고르세요 (설정 > 코어 플러그인 또는 API 키)", "errors.detail.selectApiPluginHint"),
    ("AiService가 초기화되지 않았습니다", "errors.detail.aiServiceNotInit"),
    ("TaskManager가 초기화되지 않았습니다", "errors.detail.taskManagerNotInit"),
    ("설정 잠금 실패", "errors.detail.settingsLockFailed"),
    ("설정을 불러오지 못했습니다", "errors.detail.settingsLoadFailed"),
    ("읽기 모드인 대화 — 채팅 모드로 바꾼 뒤 생성할 수 있음", "errors.detail.chatReadMode"),
    ("대화를 찾을 수 없음", "errors.detail.convNotFound"),
    ("캐릭터가 지워진 대화", "errors.detail.convDeletedCharacter"),
    ("다시 쓸 메시지가 바뀌어 자동 재생성을 멈춤", "errors.detail.rewriteMsgChanged"),
    ("다시 만들 AI 메시지가 없음", "errors.detail.noAiMsgToRegen"),
    ("응답할 사용자 메시지가 없음 (이미 응답했을 수 있음)", "errors.detail.noUserMsgToReply"),
    ("대필 결과가 비어 있음 — 다시 눌러 보세요", "errors.detail.ghostEmptyResult"),
    ("이미 대필 중", "errors.detail.ghostAlreadyRunning"),
    ("엿보기는 폰 대화에서만 됨", "errors.detail.peekPhoneOnly"),
    ("AI가 메시지를 쓰지 않았음. 다시 시도해 보세요", "errors.detail.aiWroteEmptyReply"),
    ("이미 응답을 만드는 중", "errors.detail.chatAlreadyGenerating"),
    ("응답 메시지가 없음", "errors.detail.chatNoReplyMsg"),
    ("/gen: 지시가 비어 있음", "errors.detail.slashEmptyPrompt"),
    ("보낸 프롬프트 형식이 아님", "errors.detail.slashInvalidSavedPrompt"),
    ("메시지 meta를 읽을 수 없음", "errors.detail.lintMetaReadErr"),
    ("linter가 고친 메시지가 아님", "errors.detail.lintNotFixedByLinter"),
    ("linter가 글을 고치지 않았음", "errors.detail.lintDidNotChangeText"),
    (
        "linter가 고친 뒤 메시지가 또 바뀌어 되돌리지 않음 (플러그인 후처리를 먼저 되돌리세요)",
        "errors.detail.lintEditedAfterFix",
    ),
    ("변수 이름이 비어 있음", "errors.detail.varsEmptyName"),
    ("요약을 저장하지 못함", "errors.detail.summarySaveFailed"),
    (
        "요약하는 사이 요약이 바뀌어 결과를 버렸습니다. 다시 눌러 주세요",
        "errors.detail.summaryChangedDiscarded",
    ),
    ("이 대화는 이미 요약하는 중입니다", "errors.detail.summaryAlreadyRunning"),
    ("메모리 색인이 취소되었습니다 (이미 저장한 배치는 남음)", "errors.detail.memoryIndexCancelled"),
    (
        "공급자가 0이거나 숫자가 아닌 벡터를 돌려줌 — 이 배치는 저장하지 않음",
        "errors.detail.memoryZeroVectorBatch",
    ),
    ("공급자가 0이거나 숫자가 아닌 벡터를 돌려줌", "errors.detail.memoryZeroVector"),
    ("채팅이 취소되어 회상을 멈춤", "errors.detail.memoryRecallChatCancelled"),
    ("회상이 취소됨", "errors.detail.memoryRecallCancelled"),
    ("질의 임베딩이 오래 걸려 기억 없이 보냄", "errors.detail.memoryRecallTimeout"),
    ("질의 임베딩이 비어 있음", "errors.detail.memoryEmptyQueryVec"),
    ("찾을 말을 입력하세요", "errors.detail.memoryEnterQuery"),
    ("자기 자신은 연락처에 넣을 수 없습니다", "errors.detail.phoneSelfContact"),
    ("연락처를 찾을 수 없습니다", "errors.detail.phoneContactNotFound"),
    ("프로필 생성 지시가 비어 있습니다", "errors.detail.phoneEmptyGenPrompt"),
    ("연락처 캐릭터가 지워짐", "errors.detail.phoneContactCharDeleted"),
    ("연락처 페르소나가 지워짐", "errors.detail.phoneContactPersonaDeleted"),
    ("이름이 비었음", "errors.detail.nameEmptyShort"),
    ("이름이 비어 있습니다", "errors.detail.nameEmpty"),
    ("이름을 입력하세요", "errors.detail.enterName"),
    ("이름이 너무 깁니다", "errors.detail.nameTooLong"),
    ("모델 이름이 비었습니다", "errors.detail.modelNameEmpty"),
    ("patch는 객체여야 합니다", "errors.detail.patchMustBeObject"),
    ("존재하지 않는 캐릭터입니다", "errors.detail.charDoesNotExist"),
    ("캐릭터를 찾을 수 없습니다", "errors.detail.charNotFound"),
    ("상태창 틀은 100KB까지", "errors.detail.statusTplTooLarge"),
    ("owner는 character 또는 persona", "errors.detail.ownerCharOrPersona"),
    ("png, jpg, webp, gif 이미지만 쓸 수 있습니다", "errors.detail.onlyPngJpgWebpGif"),
    ("png·jpg·webp·gif 그림만 올릴 수 있습니다", "errors.detail.onlyPngJpgWebpGifUpload"),
    ("png·jpg·webp·gif 그림만 커버로 쓸 수 있습니다", "errors.detail.onlyPngJpgWebpGifCover"),
    ("그림 파일(png·jpg·webp·gif)만 쓸 수 있습니다", "errors.detail.onlyImgFileUse"),
    ("이미지 파일(png·jpg·webp·gif·avif)만 올릴 수 있습니다", "errors.detail.onlyImgWithAvifUpload"),
    ("그림이 너무 큽니다 (8MB까지)", "errors.detail.imgTooLarge8Mb"),
    ("이미지 파일이 너무 큽니다 (8MB까지)", "errors.detail.imgFileTooLarge8Mb"),
    (
        "기본 페르소나는 지울 수 없습니다. 다른 페르소나를 기본으로 정한 뒤 지우세요",
        "errors.detail.cannotDeleteDefaultPersona",
    ),
    ("빈 메시지는 전송할 수 없습니다", "errors.detail.emptyMessageSend"),
    ("메시지가 없음", "errors.detail.msgNotFound"),
    ("월드인포 이름은 비어있을 수 없습니다", "errors.detail.worldInfoNameEmpty"),
    ("상시 항목이 아니면 트리거 키워드가 최소 1개 필요합니다", "errors.detail.worldInfoNeedsKey"),
    ("로어북 이름은 비어있을 수 없습니다", "errors.detail.lorebookNameEmpty"),
    ("이미지 키가 비어 있습니다", "errors.detail.charImgEmptyKey"),
    ("이미지 키에는 < > / \\ , 를 쓸 수 없습니다", "errors.detail.charImgInvalidKeyChars"),
    ("프리셋 이름이 비어 있습니다", "errors.detail.presetNameEmpty"),
    (
        "preset_type은 chat_completion 또는 text_completion이어야 합니다",
        "errors.detail.presetInvalidType",
    ),
    ("캐릭터·프리셋 전용 세트는 대상을 골라야 합니다", "errors.detail.qrTargetRequired"),
    ("스크립트 이름이 비어 있습니다", "errors.detail.regexNameEmpty"),
    ("적용 대상(placement)을 하나 이상 골라 주세요", "errors.detail.regexPlacementRequired"),
    ("캐릭터/프리셋 범위에는 대상 id가 필요합니다", "errors.detail.regexTargetIdRequired"),
    ("찾을 패턴이 비어 있습니다", "errors.detail.regexEmptyPattern"),
    ("오디오 파일(mp3·wav·ogg·m4a·flac)만 쓸 수 있습니다", "errors.detail.audioOnlyExts"),
    ("오디오 파일이 너무 큽니다 (5MB까지)", "errors.detail.audioTooLarge5Mb"),
    ("아직 만든 음성이 없음 (채팅 모드에서 읽기를 누르면 만들어짐)", "errors.detail.ttsNoAudioYet"),
    ("읽을 글이 없습니다", "errors.detail.ttsNoText"),
    ("ElevenLabs 목소리를 먼저 고르세요 (설정 > 음성 읽기)", "errors.detail.ttsSelectElevenVoice"),
    ("이 메시지에는 그림 프롬프트가 없습니다", "errors.detail.imgMsgNoPrompt"),
    ("그릴 내용을 입력하세요", "errors.detail.imgEnterPrompt"),
    ("NovelAI 응답에 그림이 없습니다", "errors.detail.naiNoImage"),
    ("이미지 응답에 그림이 없습니다", "errors.detail.imgRespNoImage"),
    ("이미지 응답 형식을 알 수 없습니다", "errors.detail.imgUnknownRespFormat"),
    ("그림을 찾을 수 없습니다", "errors.detail.imgNotFound"),
    ("그림 폴더가 없습니다", "errors.detail.imgFolderMissing"),
    ("갤러리의 그림만 이름을 바꿀 수 있습니다", "errors.detail.imgGalleryOnlyRename"),
    ("갤러리의 그림에만 태그를 달 수 있습니다", "errors.detail.imgGalleryOnlyTag"),
    ("갤러리의 그림만 지울 수 있습니다", "errors.detail.imgGalleryOnlyDelete"),
    ("잘못된 시메지 ID 또는 슬롯입니다", "errors.detail.mascotInvalidSlot"),
    ("시메지에 연결된 API가 없습니다. 설정에서 API를 선택해 주세요.", "errors.detail.mascotNoApi"),
    ("시메지를 찾을 수 없습니다", "errors.detail.mascotNotFound"),
    ("시메지끼리 AI 대화가 꺼져 있습니다", "errors.detail.mascotPeerAiOff"),
    ("첫 번째 시메지를 찾을 수 없습니다", "errors.detail.mascotFirstNotFound"),
    ("두 번째 시메지를 찾을 수 없습니다", "errors.detail.mascotSecondNotFound"),
    ("태그 생성 실패", "errors.detail.musicTagCreateFailed"),
    ("삭제할 데이터가 존재하지 않음", "errors.detail.watcherDeleteMissing"),
    ("HTML이 너무 큽니다 (1MB까지)", "errors.detail.htmlTooLarge1Mb"),
    ("글꼴 파일(ttf·otf·woff·woff2)만 올릴 수 있습니다", "errors.detail.fontAllowedExts"),
    (
        "글꼴 파일이 아니거나 읽을 수 없는 형식입니다 (글꼴 묶음 .ttc는 지원하지 않음)",
        "errors.detail.fontTtcUnsupported",
    ),
    ("글꼴 파일이 아니거나 읽을 수 없는 형식입니다", "errors.detail.fontInvalidFormat"),
    ("이어 붙일 수 없는 파일입니다", "errors.detail.cannotAppendFile"),
    ("파일을 찾을 수 없습니다", "errors.detail.fileNotFound"),
    ("파일이 아닙니다", "errors.detail.notAFile"),
    ("빈 파일입니다", "errors.detail.emptyFile"),
    ("파일이 너무 큽니다 (10MB 이하)", "errors.detail.fileTooLarge10Mb"),
    ("파일이 너무 큽니다 (50MB 이하)", "errors.detail.fileTooLarge50Mb"),
    ("임시 저장 파일이 아닙니다", "errors.detail.notStagedFile"),
    ("Sandcastle 백업 파일이 아닙니다", "errors.detail.notSandcastleBackup"),
    (
        "더 새 버전의 앱에서 만든 백업입니다. 앱을 업데이트해 주세요",
        "errors.detail.backupFromNewerApp",
    ),
    ("PNG가 아님", "errors.detail.notPngShort"),
    ("PNG가 잘려 있음", "errors.detail.pngTruncated"),
    ("PNG 파일이 아닙니다", "errors.detail.notPngFile"),
    ("PNG 청크가 손상되었습니다", "errors.detail.pngChunkCorrupted"),
    ("PNG에 캐릭터 카드 데이터(chara/ccv3)가 없습니다", "errors.detail.pngNoCharaData"),
    ("캐릭터 이름이 없습니다", "errors.detail.cardMissingCharName"),
    ("내보낼 캐릭터를 골라 주세요", "errors.detail.packSelectCharacter"),
    ("Sandcastle 캐릭터 팩이 아닙니다", "errors.detail.notSandcastlePack"),
    ("더 새 버전의 앱에서 만든 팩입니다. 앱을 업데이트해 주세요", "errors.detail.packFromNewerApp"),
    ("휴지통에 없는 항목", "errors.detail.trashItemMissing"),
    ("휴지통 항목을 읽지 못함", "errors.detail.trashReadErr"),
    ("모르는 기기", "errors.detail.syncUnknownPeer"),
    ("자동 동기화가 꺼져 있습니다", "errors.detail.syncAutoDisabled"),
    ("이 빌드에는 구글 드라이브 연결이 들어 있지 않음", "errors.detail.syncNoDriveInBuild"),
    ("구글 드라이브에 먼저 로그인하세요 (설정 > 동기화)", "errors.detail.syncLoginDriveFirst"),
    ("동기화 폴더를 먼저 고르세요 (설정 > 동기화)", "errors.detail.syncSelectFolderFirst"),
    ("동기화가 이미 진행 중입니다", "errors.detail.syncAlreadyRunning"),
    ("동기화 묶음이 아님 (manifest.json 없음)", "errors.detail.syncNotBundleNoManifest"),
    ("settings.json 모양이 이상함", "errors.detail.syncInvalidSettingsJson"),
    ("파일 해시가 이상함", "errors.detail.syncBadFileHash"),
    ("이 기기에서 만든 묶음은 합치지 않음", "errors.detail.syncOwnBundleSkipped"),
    ("이미 합친 묶음이거나 그보다 오래된 묶음", "errors.detail.syncAlreadyMergedOrOlder"),
    ("이미 고른 충돌이거나 없는 충돌", "errors.detail.conflictNotFound"),
    ("이 항목은 버리기(확인함)만 할 수 있음", "errors.detail.conflictDismissOnly"),
    ("충돌의 키 모양이 이상함", "errors.detail.conflictBadKey"),
    ("이 기기에 그 항목이 더는 없음 — 버리기를 고르세요", "errors.detail.conflictLocalGone"),
    ("받은 행을 읽지 못함", "errors.detail.conflictReadRemoteErr"),
    ("빈 토큰", "errors.detail.driveEmptyToken"),
    ("구글 드라이브 권한을 다시 받아야 함", "errors.detail.driveReauthRequired"),
    ("구글 로그인을 5분 안에 마치지 않아 그만둠", "errors.detail.driveLoginTimeout"),
    (
        "구글이 refresh token을 주지 않음 — 연결 끊기 후 다시 로그인",
        "errors.detail.driveNoRefreshToken",
    ),
    ("구글 로그인 응답이 이 요청과 맞지 않음 (state)", "errors.detail.driveStateMismatch"),
    (
        "구글 드라이브가 연결되어 있지 않음 (설정 > 동기화에서 로그인)",
        "errors.detail.driveNotConnected",
    ),
    (
        "구글 드라이브 연결이 끊어졌음 (권한이 취소됐거나 만료) — 설정 > 동기화에서 다시 로그인",
        "errors.detail.driveDisconnected",
    ),
    ("채팅 파일의 첫 줄(헤더)을 읽을 수 없습니다", "errors.detail.stChatHeaderErr"),
    (
        "SillyTavern 데이터 폴더가 아닙니다 (characters·worlds·chats 폴더나 settings.json이 없음)",
        "errors.detail.stNotDataFolder",
    ),
    ("부모와 같은 메시지가 없음", "errors.detail.stBranchNoCommonMsg"),
    (
        "SillyTavern 캐릭터 카드·프리셋·정규식·월드인포 중 어느 것도 아닙니다",
        "errors.detail.stUnrecognizedJson",
    ),
    ("채팅에 메시지가 없습니다", "errors.detail.stChatEmpty"),
    (
        "어떤 캐릭터의 채팅인지 찾지 못했습니다. 캐릭터를 먼저 가져오거나 직접 골라 주세요",
        "errors.detail.stChatUnknownOwner",
    ),
    ("가져올 수 있는 정규식이 없습니다", "errors.detail.stNoRegexToImport"),
    ("SillyTavern 빠른 답장 세트가 아닙니다 (qrList가 없음)", "errors.detail.stNotQrSet"),
    ("이 게임의 세션이 아닙니다", "errors.detail.gameWrongSession"),
    ("이 게임의 대화가 아닙니다", "errors.detail.gameWrongConversation"),
    ("세이브가 256KB를 초과했습니다", "errors.detail.gameSaveTooLarge"),
    ("진행 중인 세션이 없습니다", "errors.detail.gameNoActiveSession"),
    ("열려 있는 게임이 없습니다 (open_game을 먼저 부르세요)", "errors.detail.gameNotOpenedCall"),
    ("응답을 기다리는 동안 세션이 바뀌었습니다", "errors.detail.gameSessionChangedWait"),
    ("지금 열려 있는 게임은 지울 수 없습니다", "errors.detail.gameCannotDeleteOpen"),
    ("지금 플레이 중인 게임은 지울 수 없습니다", "errors.detail.gameCannotDeletePlaying"),
    ("게임 id가 올바르지 않습니다", "errors.detail.gameInvalidId"),
    ("게임 폴더를 찾을 수 없습니다", "errors.detail.gameFolderNotFound"),
    ("게임을 찾을 수 없습니다", "errors.detail.gameNotFound"),
    (
        "알 수 없는 파일 형식입니다 (이미지: png/jpg/webp/gif/avif, 오디오: mp3/ogg/opus/wav/m4a/aac/flac)",
        "errors.detail.gameUnknownFileKind",
    ),
    ("설명/대사가 너무 깁니다", "errors.detail.gameDescTooLong"),
    ("효과음에는 tag(소리 그룹 이름)가 필요합니다", "errors.detail.gameSfxNeedsTag"),
    ("효과음 tag에 < > \" , 줄바꿈은 쓸 수 없고 40자까지입니다", "errors.detail.gameSfxInvalidTag"),
    ("캐릭터 이미지에는 캐릭터(gameCharacterId)가 필요합니다", "errors.detail.gameCharImgNeedsChar"),
    ("스탠딩에는 감정명(emotion)이 필요합니다", "errors.detail.gameStandingNeedsEmotion"),
    ("감정명에 < > : \" , 줄바꿈은 쓸 수 없고 40자까지입니다", "errors.detail.gameInvalidEmotionName"),
    ("캐릭터 이미지의 usage는 standing 또는 map이어야 합니다", "errors.detail.gameInvalidCharUsage"),
    ("존재하지 않는 위치입니다", "errors.detail.gameLocNotFound"),
    ("존재하지 않는 시간대입니다", "errors.detail.gameTimeNotFound"),
    ("존재하지 않는 날씨입니다", "errors.detail.gameWeatherNotFound"),
    ("settingsJson은 객체여야 합니다", "errors.detail.gameSettingsJsonObj"),
    ("settingsJson이 너무 큽니다", "errors.detail.gameSettingsJsonTooLarge"),
    ("에셋 폴더가 게임 폴더 밖을 가리킵니다", "errors.detail.gameAssetOutsideDir"),
    (
        "에셋 키는 영문 소문자, 숫자, . _ - 만 쓸 수 있고 64자까지입니다",
        "errors.detail.gameInvalidAssetKey",
    ),
    ("에셋 경로가 올바르지 않습니다", "errors.detail.gameInvalidAssetPath"),
    (
        "파일을 복사하는 동안 내용이 바뀌었습니다. 다시 시도하세요",
        "errors.detail.gameFileChangedCopying",
    ),
    ("추가한 에셋을 다시 읽지 못했습니다", "errors.detail.gameReloadAssetFailed"),
    ("이 게임의 에셋이 아닙니다", "errors.detail.gameWrongAssetOwner"),
    ("제목 길이가 올바르지 않습니다", "errors.detail.gameInvalidTitleLen"),
    ("템플릿 패키지가 없습니다", "errors.detail.gameTemplatePkgMissing"),
    ("게임 패키지 파일을 찾을 수 없습니다", "errors.detail.gamePkgFileNotFound"),
    ("게임 파일을 찾을 수 없습니다", "errors.detail.gameFileMissing"),
    ("호출이 너무 잦습니다. 잠시 후 다시 시도하세요", "errors.detail.gameRateLimited"),
    ("이 캐릭터와의 대화가 열려 있지 않습니다", "errors.detail.gameNoActiveCharDialogue"),
    ("이 게임에 속한 캐릭터가 아닙니다", "errors.detail.gameCharNotInGame"),
    ("입력 길이가 올바르지 않습니다", "errors.detail.gameInvalidInputLen"),
    ("열린 씬이 없어 씬 입력을 쓸 수 없습니다", "errors.detail.gameNoSceneForChat"),
    (
        "이 씬에서는 씬 입력을 쓸 수 없습니다 (엔딩 씬이거나 AI 이어가기가 꺼진 씬)",
        "errors.detail.gameSceneChatDisabled",
    ),
    ("열려 있는 씬이 없습니다", "errors.detail.gameNoOpenScene"),
    ("패키지 크기가 너무 큽니다", "errors.detail.gamePkgTooLarge"),
    ("잘못된 경로가 포함된 패키지입니다", "errors.detail.gamePkgUnsafePath"),
    ("경로 탈출이 감지됐습니다", "errors.detail.gamePathTraversal"),
    ("manifest.json이 없습니다", "errors.detail.gameMissingManifest"),
    ("data/definition.json이 없습니다", "errors.detail.gameMissingDefinition"),
    ("이 게임은 지도/이동 기능을 쓰지 않습니다", "errors.detail.gameCapNoMap"),
    ("씬이 진행 중일 때는 이동할 수 없습니다", "errors.detail.gameBlockedMoveInScene"),
    ("현재 위치가 없어 이동할 수 없습니다", "errors.detail.gameBlockedNoCurrentLoc"),
    ("그 경로로는 이동할 수 없습니다", "errors.detail.gameBlockedNoRoute"),
    ("이동 조건이 충족되지 않았습니다", "errors.detail.gameBlockedMoveCond"),
    ("이 게임은 위치 기능을 쓰지 않습니다", "errors.detail.gameCapNoLocation"),
    ("씬이 진행 중일 때는 씬 입력(sceneSay)을 사용하세요", "errors.detail.gameUseSceneSay"),
    ("그 캐릭터는 지금 여기에 없습니다", "errors.detail.gameCharNotHere"),
    ("열린 대화가 없습니다", "errors.detail.gameNoOpenDialogue"),
    ("지금 열려 있는 씬의 선택지가 아닙니다", "errors.detail.gameChoiceWrongScene"),
    ("지금은 고를 수 없는 선택지입니다", "errors.detail.gameChoiceUnavailable"),
    ("엔딩 씬은 끝낼 수 없습니다", "errors.detail.gameEndingCannotClose"),
    ("이 게임은 시간 기능을 쓰지 않습니다", "errors.detail.gameCapNoTime"),
    ("minutes는 음수일 수 없습니다", "errors.detail.gameNegativeMinutes"),
    ("가지고 있지 않은 아이템입니다", "errors.detail.gameItemNotOwned"),
    ("사용할 수 없는 아이템입니다", "errors.detail.gameItemNotUsable"),
    ("지금은 사용할 수 없습니다", "errors.detail.gameCannotUseNow"),
    ("핵심 아이템은 버릴 수 없습니다", "errors.detail.gameKeyItemNoDrop"),
    ("버릴 수 있는 아이템이 없습니다", "errors.detail.gameNoDroppableItem"),
    ("제작자가 StoryPlan에 맡긴 manual 이벤트가 아닙니다", "errors.detail.gameEventNotPlanUsable"),
    ("지금은 이 이벤트를 발동할 수 없습니다", "errors.detail.gameEventBlockedNow"),
    ("제작자가 StoryPlan에 맡긴 씬이 아닙니다", "errors.detail.gameSceneNotPlanUsable"),
    ("이미 열려 있거나 대기 중인 씬입니다", "errors.detail.gameSceneAlreadyQueued"),
    ("이 제안은 이미 무대에 올렸습니다", "errors.detail.gameSuggestionAlreadyStaged"),
    ("이 책갈피에는 복원할 상태 스냅샷이 없습니다", "errors.detail.gameSaveNoSnapshot"),
    ("이 시점에는 분기할 상태가 남아 있지 않습니다", "errors.detail.gameNoBranchSnapshot"),
    ("열린 게임과 진행 중인 세션이 다릅니다", "errors.detail.gameSessionMismatchOpen"),
    ("kind는 event 또는 scene이어야 합니다", "errors.detail.gameSuggestionInvalidKind"),
    ("이 게임은 StoryPlan을 쓰지 않습니다", "errors.detail.gameStoryPlanDisabled"),
    ("아직 세워진 플랜이 없습니다", "errors.detail.gameNoPlanYet"),
    ("지금 플랜이 제안한 것이 아닙니다", "errors.detail.gameNotInCurrentPlan"),
    ("요청한 데이터를 찾을 수 없습니다", "errors.detail.dbRowNotFound"),
    ("데이터베이스가 사용 중입니다", "errors.detail.dbLockedBusy"),
    ("작업이 취소되었습니다", "errors.detail.taskCancelledMsg"),
    ("작업을 멈췄습니다", "errors.detail.taskStoppedMsg"),
    ("앱이 종료되어 작업이 중단되었습니다", "errors.detail.taskInterruptedShutdown"),
    ("작업 실행 중 패닉이 발생했습니다", "errors.detail.taskPanicked"),
    ("작업 결과를 받지 못했습니다", "errors.detail.taskNoResultReceived"),
    ("테마가 저장됨 폴더 경로를 가져오는 데 실패함", "errors.detail.themeDirErr"),
    ("파일 경로를 읽는 데 실패함", "errors.detail.themeReadDirErr"),
    ("유효하지 않은 파일명입니다.", "errors.detail.themeInvalidFilename"),
    ("파일을 읽는 중 실패했습니다.", "errors.detail.themeReadFileErr"),
    ("파일을 저장하는 중 실패했습니다.", "errors.detail.themeSaveFileErr"),
    ("파일을 삭제하는 중 실패했습니다.", "errors.detail.themeDeleteFileErr"),
];

const TEMPLATE_RULES: &[TemplateRule] = &[
    TemplateRule {
        key: "errors.detail.outputTruncated",
        segs: &[
            "",
            "이(가) 출력 한도(max_tokens)에서 잘려 끊김 — 실행하지 않음. 출력 한도를 늘려 다시 시도하세요 (이미 쓴 토큰은 사용량에 기록됨)",
        ],
        args: &["what"],
    },
    TemplateRule {
        key: "errors.detail.missingApiKeyOnDevice",
        segs: &[
            "이 기기에 '",
            "' 프로필의 API 키가 없습니다. 설정 > API 키에서 넣어 주세요 (키는 기기마다 따로 저장되고 백업·동기화되지 않음)",
        ],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.manifestApiTooOld",
        segs: &[
            "이 앱이 더는 지원하지 않는 오래된 플러그인 API (apiVersion ",
            " < ",
            ") — 플러그인을 새 판으로 업데이트하세요",
        ],
        args: &["ver", "min"],
    },
    TemplateRule {
        key: "errors.detail.contextOverflowFixed",
        segs: &[
            "고정 프롬프트(",
            " 토큰)가 사용 가능한 컨텍스트(",
            " 토큰)를 넘습니다. 컨텍스트 크기를 늘리거나 응답 길이를 줄여 주세요",
        ],
        args: &["fixed", "budget"],
    },
    TemplateRule {
        key: "errors.detail.gameInvalidAssetCategory",
        segs: &["에셋 카테고리가 올바르지 않습니다: ", " (voice, sfx, bgm, character, cg, background)"],
        args: &["cat"],
    },
    TemplateRule {
        key: "errors.detail.manifestApiTooNew",
        segs: &["이 앱이 지원하는 플러그인 API보다 새것 (apiVersion ", " > ", ") — 앱을 업데이트하세요"],
        args: &["ver", "max"],
    },
    TemplateRule {
        key: "errors.detail.syncNewerDbWithApp",
        segs: &["더 새 앱에서 만든 묶음 (DB 버전 ", " > 이 기기 ", ") — 이 기기의 앱(", ")을 업데이트하세요"],
        args: &["remote", "local", "app"],
    },
    TemplateRule {
        key: "errors.detail.budgetExceededAuto",
        segs: &["이번 달 예산을 넘어 실행하지 않았습니다 ($", " / $", "). 자동 실행은 실행하지 않았습니다."],
        args: &["used", "limit"],
    },
    TemplateRule {
        key: "errors.detail.trashPluginIdConflict",
        segs: &["같은 id('", "')의 플러그인이 이미 설치되어 있어 되살릴 수 없음 — 그것을 먼저 지우세요"],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.syncNewerDb",
        segs: &["더 새 앱에서 만든 묶음 (DB 버전 ", " > 이 기기 ", ") — 이 기기의 앱을 업데이트하세요"],
        args: &["remote", "local"],
    },
    TemplateRule {
        key: "errors.detail.manifestInvalidSettingType",
        segs: &["settings.", "의 type은 text·number·toggle·select 중 하나"],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.zipTooManyImages",
        segs: &["ZIP에 그림이 ", "장 있어 넣을 수 없습니다 (한 번에 ", "장까지). 나눠서 넣어 주세요"],
        args: &["total", "max"],
    },
    TemplateRule {
        key: "errors.detail.memoryOverCap",
        segs: &["예상 ", "토큰이 실행당 상한(", ")을 넘음 — 상한을 늘리거나 넘겨서 진행을 확인하세요"],
        args: &["est", "cap"],
    },
    TemplateRule {
        key: "errors.detail.backupIncompatibleMigration",
        segs: &["DB 구조가 다른 빌드에서 만든 백업이라 복원할 수 없습니다 (마이그레이션 ", ")"],
        args: &["ver"],
    },
    TemplateRule {
        key: "errors.detail.syncDownloadInterrupted",
        segs: &["파일을 받다가 멈춤 — 다시 동기화하면 받은 파일은 건너뛰고 이어서 받음 (", ")"],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.pkgDisallowedFileKind",
        segs: &["넣을 수 없는 파일 종류: ", " (js·css·html·json·그림·글꼴만)"],
        args: &["path"],
    },
    TemplateRule {
        key: "errors.detail.structuredInvalidName",
        segs: &["구조화 출력 이름이 올바르지 않음 (영문·숫자·_·- 64자 이하): '", "'"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.trashMissingParent",
        segs: &["연결된 항목이 없어 되살릴 수 없음 (", ") — 먼저 그 항목을 되살리세요"],
        args: &["missing"],
    },
    TemplateRule {
        key: "errors.detail.syncNewerFormat",
        segs: &["더 새 앱에서 만든 묶음 (형식 v", ") — 이 기기의 앱을 업데이트하세요"],
        args: &["ver"],
    },
    TemplateRule {
        key: "errors.detail.gameNeedsNewerEngine",
        segs: &["이 게임은 앱 ", " 이상이 필요합니다 (지금 ", "). 앱을 업데이트해 주세요"],
        args: &["min", "cur"],
    },
    TemplateRule {
        key: "errors.detail.proposalTooManyPending",
        segs: &["기다리는 제안이 너무 많음 (", "개까지) — 사용자가 고르기를 기다리세요"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.toolsInvalidName",
        segs: &["도구 이름이 올바르지 않음 (영문·숫자·_·- 64자 이하): '", "'"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.manifestPanelPageHtml",
        segs: &["panels.", "의 page는 플러그인 폴더 안의 .html이어야 함"],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.backupUnknownDbVer",
        segs: &["이 앱에 없는 DB 버전(", ")의 백업입니다. 앱을 업데이트해 주세요"],
        args: &["ver"],
    },
    TemplateRule {
        key: "errors.detail.stZipTempExtractErr",
        segs: &["임시 폴더에 풀지 못했습니다 (디스크 공간이 모자랄 수 있음): ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.gameCategoryWrongKind",
        segs: &["이 카테고리에는 ", " 파일만 넣을 수 있습니다 (받은 파일: ", ")"],
        args: &["expected", "got"],
    },
    TemplateRule {
        key: "errors.detail.summaryFailedReason",
        segs: &["요약하지 못함 (", ") — 실행 기록에서 이유를 볼 수 있습니다"],
        args: &["reason"],
    },
    TemplateRule {
        key: "errors.detail.manifestSelectNeedsOptions",
        segs: &["settings.", ": select에는 options가 필요함"],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.phoneInvalidContactKind",
        segs: &["종류는 persona 또는 character여야 합니다: ", ""],
        args: &["kind"],
    },
    TemplateRule {
        key: "errors.detail.budgetExceededUser",
        segs: &["이번 달 예산을 넘어 실행하지 않았습니다 ($", " / $", ")."],
        args: &["used", "limit"],
    },
    TemplateRule {
        key: "errors.detail.postprocessNotesOnly",
        segs: &["'", "'은(는) 표시만 하도록 되어 있어 글을 바꿀 수 없음"],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.manifestMainJs",
        segs: &["main은 플러그인 폴더 안의 .js 파일이어야 함: ", ""],
        args: &["main"],
    },
    TemplateRule {
        key: "errors.detail.gameBgmInvalidMood",
        segs: &["배경음 tag(분위기)가 올바르지 않습니다 (가능: ", ")"],
        args: &["moods"],
    },
    TemplateRule {
        key: "errors.detail.taskStateAlreadyChanged",
        segs: &["작업 상태가 이미 바뀌었습니다 (기대: ", ", 현재: ", ")"],
        args: &["from", "cur"],
    },
    TemplateRule {
        key: "errors.detail.storageReservedPrefix",
        segs: &["'", "'로 시작하는 저장소 키는 앱이 쓰므로 쓸 수 없음"],
        args: &["prefix"],
    },
    TemplateRule {
        key: "errors.detail.embedBatchLimit",
        segs: &["한 번에 ", "개까지 임베딩할 수 있음 — 나눠서 부르세요"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.pluginDisabledMsg",
        segs: &["'", "' 기능이 꺼져 있습니다 (설정 > 코어 플러그인)"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.vertexInvalidCred",
        segs: &["Vertex 인증 정보 형식이 올바르지 않습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.embedNthEmpty",
        segs: &["", "번째 글이 비어 있음 — 빈 글은 임베딩할 수 없음"],
        args: &["n"],
    },
    TemplateRule {
        key: "errors.detail.musicCoverTooLarge",
        segs: &["파일 크기가 너무 큼 (1MB 초과): ", " bytes"],
        args: &["bytes"],
    },
    TemplateRule {
        key: "errors.detail.gameAssetInUse",
        segs: &["다른 곳에서 쓰는 에셋이라 지울 수 없습니다: [", "]"],
        args: &["refs"],
    },
    TemplateRule {
        key: "errors.detail.manifestInvalidCommandId",
        segs: &["commands의 id가 올바르지 않거나 겹침: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.toolsChoiceMissing",
        segs: &["tool_choice가 없는 도구를 가리킴: '", "'"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.backupRestoreFailed",
        segs: &["백업 복원 적용 실패 (원래 데이터로 되돌림): ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.gameDataInvalidMsg",
        segs: &["게임 데이터에 문제가 있어 시작할 수 없습니다: ", ""],
        args: &["msg"],
    },
    TemplateRule {
        key: "errors.detail.storageFull",
        segs: &["플러그인 저장소가 가득 참 (키 ", "개, ", "MB까지)"],
        args: &["maxKeys", "maxMb"],
    },
    TemplateRule {
        key: "errors.detail.communityMoveFolderErr",
        segs: &["플러그인 폴더를 새 배치로 옮기지 못함 (", "): ", ""],
        args: &["name", "err"],
    },
    TemplateRule {
        key: "errors.detail.communityAiInputTooLong",
        segs: &["AI에 보낼 글이 너무 김 (", "자까지, 지금 ", "자)"],
        args: &["max", "cur"],
    },
    TemplateRule {
        key: "errors.detail.varsInvalidChars",
        segs: &["변수 이름에 '::'·'}}'는 쓸 수 없음: ", ""],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.presetInvalidBlockPos",
        segs: &["블록 '", "'의 position이 올바르지 않습니다"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.driveSaveKeyringErr",
        segs: &["드라이브 연결을 저장하지 못함 (키 저장소): ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.manifestInvalidId",
        segs: &["id는 소문자·숫자·'-'로 2~", "자여야 함: ", ""],
        args: &["max", "id"],
    },
    TemplateRule {
        key: "errors.detail.httpTimeoutDetail",
        segs: &["AI 서버 응답 시간이 초과되었습니다 (", "): ", ""],
        args: &["status", "detail"],
    },
    TemplateRule {
        key: "errors.detail.manifestInvalidPanelId",
        segs: &["panels의 id가 올바르지 않거나 겹침: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.driveSaveLoginErr",
        segs: &["구글 로그인을 저장하지 못함 (키 저장소): ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.androidKeyringErr",
        segs: &["Android 키 저장소를 열 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.manifestMaxPanelsCommands",
        segs: &["panels는 ", "개, commands는 ", "개까지"],
        args: &["maxPanels", "maxCmds"],
    },
    TemplateRule {
        key: "errors.detail.capUnsupportedFormat",
        segs: &["이 API 형식은 ", "을(를) 지원하지 않음: ", ""],
        args: &["feature", "reason"],
    },
    TemplateRule {
        key: "errors.detail.taskInvalidStateTransition",
        segs: &["작업 상태를 ", "에서 ", "(으)로 바꿀 수 없습니다"],
        args: &["from", "to"],
    },
    TemplateRule {
        key: "errors.detail.manifestInvalidSettingKey",
        segs: &["settings의 key가 올바르지 않음: ", ""],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.syncRepoHashMismatch",
        segs: &["동기화 저장소의 파일 내용이 해시와 다름: ", ""],
        args: &["path"],
    },
    TemplateRule {
        key: "errors.detail.driveTokenHttpErr",
        segs: &["구글 로그인 토큰을 받지 못함 (HTTP ", ")"],
        args: &["status"],
    },
    TemplateRule {
        key: "errors.detail.taskExecutorNotRegistered",
        segs: &["", " 작업을 처리할 실행기가 등록되지 않았습니다"],
        args: &["taskType"],
    },
    TemplateRule {
        key: "errors.detail.httpServerErrorDetail",
        segs: &["AI 서버에서 오류가 발생했습니다 (", "): ", ""],
        args: &["status", "detail"],
    },
    TemplateRule {
        key: "errors.detail.toolsInvalidSchema",
        segs: &["도구 '", "'의 인자 스키마를 쓸 수 없음: ", ""],
        args: &["name", "problems"],
    },
    TemplateRule {
        key: "errors.detail.httpTimeout",
        segs: &["AI 서버 응답 시간이 초과되었습니다 (", ")"],
        args: &["status"],
    },
    TemplateRule {
        key: "errors.detail.varsDefaultNotBool",
        segs: &["'", "'의 처음 값은 true 또는 false"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.driveRefreshHttpErr",
        segs: &["구글 토큰을 새로 받지 못함 (HTTP ", ")"],
        args: &["status"],
    },
    TemplateRule {
        key: "errors.detail.stZipTooLargeGb",
        segs: &["ZIP이 ", "GB보다 커서 가져올 수 없습니다"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.gameInvalidEngineVer",
        segs: &["패키지의 엔진 버전을 읽을 수 없습니다: ", ""],
        args: &["ver"],
    },
    TemplateRule {
        key: "errors.detail.taskUnknownAttemptStatusDb",
        segs: &["알 수 없는 attempt status: ", ""],
        args: &["val"],
    },
    TemplateRule {
        key: "errors.detail.httpModelNotFoundDetail",
        segs: &["요청한 모델을 찾을 수 없습니다 (", "): ", ""],
        args: &["status", "detail"],
    },
    TemplateRule {
        key: "errors.detail.httpRateLimitDetail",
        segs: &["API 요청 한도를 초과했습니다 (", "): ", ""],
        args: &["status", "detail"],
    },
    TemplateRule {
        key: "errors.detail.regexInvalidScript",
        segs: &["정규식 '", "'이(가) 올바르지 않습니다: ", ""],
        args: &["name", "reason"],
    },
    TemplateRule {
        key: "errors.detail.stChatNthMsgErr",
        segs: &["채팅 ", "번째 메시지를 읽을 수 없습니다: ", ""],
        args: &["n", "err"],
    },
    TemplateRule {
        key: "errors.detail.communityAiPerMinLimit",
        segs: &["AI 호출이 너무 잦음 (1분에 ", "번까지)"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.presetInvalidBlockRole",
        segs: &["블록 '", "'의 role이 올바르지 않습니다"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.naiOpenErr",
        segs: &["NovelAI 응답을 열 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.stZipTooManyEntries",
        segs: &["ZIP에 항목이 너무 많습니다 (", "개까지)"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.capUnsupportedConn",
        segs: &["'", "' 연결은 ", "을(를) 지원하지 않음: ", ""],
        args: &["profile", "feature", "reason"],
    },
    TemplateRule {
        key: "errors.detail.manifestSettingLabelLen",
        segs: &["settings.", "의 label은 1~", "자"],
        args: &["key", "max"],
    },
    TemplateRule {
        key: "errors.detail.manifestCommandTitleLen",
        segs: &["commands.", "의 title은 1~", "자"],
        args: &["id", "max"],
    },
    TemplateRule {
        key: "errors.detail.pkgFolderTooManyEntries",
        segs: &["폴더 안 항목이 너무 많음 (최대 ", "개)"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.postprocessInvalidMode",
        segs: &["후처리 방식은 notes / fix: ", ""],
        args: &["mode"],
    },
    TemplateRule {
        key: "errors.detail.httpServerError",
        segs: &["AI 서버에서 오류가 발생했습니다 (", ")"],
        args: &["status"],
    },
    TemplateRule {
        key: "errors.detail.imgMaxMbLimit",
        segs: &["이미지는 ", "MB 이하만 넣을 수 있습니다"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.regexNotFound",
        segs: &["정규식 스크립트를 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.appDataDirErr",
        segs: &["앱 데이터 폴더를 찾을 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.cardBase64Err",
        segs: &["카드 데이터 base64 해석 실패: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.conflictRollbackFailed",
        segs: &["되돌리기 실패 (원래 데이터로 둠): ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.driveLoginDenied",
        segs: &["구글 로그인이 취소되었거나 거절됨 (", ")"],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.updateCheckHttpErr",
        segs: &["새 버전을 확인하지 못함 (HTTP ", ")"],
        args: &["status"],
    },
    TemplateRule {
        key: "errors.detail.taskCannotActionInState",
        segs: &["", " 상태의 작업은 ", "할 수 없습니다 (", ")"],
        args: &["status", "action", "id"],
    },
    TemplateRule {
        key: "errors.detail.communityMissingPerm",
        segs: &["권한이 없음: ", "에는 '", "' 권한이 필요"],
        args: &["method", "perm"],
    },
    TemplateRule {
        key: "errors.detail.pluginManifestPrefix",
        segs: &["플러그인 manifest.json: ", ""],
        args: &["msg"],
    },
    TemplateRule {
        key: "errors.detail.pkgZipTooManyEntries",
        segs: &["zip 항목이 너무 많음 (최대 ", "개)"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.httpModelNotFound",
        segs: &["요청한 모델을 찾을 수 없습니다 (", ")"],
        args: &["status"],
    },
    TemplateRule {
        key: "errors.detail.httpRateLimit",
        segs: &["API 요청 한도를 초과했습니다 (", ")"],
        args: &["status"],
    },
    TemplateRule {
        key: "errors.detail.structuredInvalidSchema",
        segs: &["구조화 출력 스키마를 쓸 수 없음: ", ""],
        args: &["problems"],
    },
    TemplateRule {
        key: "errors.detail.worldInfoEntryNotFound",
        segs: &["월드인포 항목을 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.charImgDuplicateKey",
        segs: &["이 판에 이미 같은 키가 있습니다: ", ""],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.fontTooLargeMb",
        segs: &["글꼴 파일이 너무 큽니다 (", "MB까지)"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.invalidJsonFile",
        segs: &["JSON 파일을 읽을 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.packReadCharErr",
        segs: &["캐릭터 데이터를 읽을 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.packReadMsgErr",
        segs: &["메시지 데이터를 읽을 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.syncEncryptedUnsupported",
        segs: &["암호화된 묶음(", ")은 아직 열 수 없음"],
        args: &["enc"],
    },
    TemplateRule {
        key: "errors.detail.stZipUnpackedTooLargeGb",
        segs: &["가져올 파일만 풀어도 ", "GB가 넘습니다"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.gameEntityNotFound",
        segs: &["존재하지 않는 상호작용 대상입니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.taskUnknownStatusDb",
        segs: &["알 수 없는 task status: ", ""],
        args: &["val"],
    },
    TemplateRule {
        key: "errors.detail.manifestPanelTitleLen",
        segs: &["panels.", "의 title은 1~", "자"],
        args: &["id", "max"],
    },
    TemplateRule {
        key: "errors.detail.httpAuthFailedDetail",
        segs: &["API 인증에 실패했습니다 (", "): ", ""],
        args: &["status", "detail"],
    },
    TemplateRule {
        key: "errors.detail.httpRejectedDetail",
        segs: &["AI 요청이 거부되었습니다 (", "): ", ""],
        args: &["status", "detail"],
    },
    TemplateRule {
        key: "errors.detail.structuredOutputMismatch",
        segs: &["응답이 '", "' 스키마에 맞지 않음: ", ""],
        args: &["name", "problems"],
    },
    TemplateRule {
        key: "errors.detail.syncReadFromFolderErr",
        segs: &["동기화 폴더에서 읽지 못함 (", "): ", ""],
        args: &["name", "err"],
    },
    TemplateRule {
        key: "errors.detail.storageValTooLarge",
        segs: &["저장소 값이 너무 큼 (최대 ", "KB)"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.manifestDuplicateSettingKey",
        segs: &["settings의 key가 겹침: ", ""],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.unsupportedApiFormat",
        segs: &["지원하지 않는 API 형식입니다: ", ""],
        args: &["format"],
    },
    TemplateRule {
        key: "errors.detail.aiConnectFailed",
        segs: &["AI 서버에 연결할 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.vertexInvalidSa",
        segs: &["서비스 계정을 읽을 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.varsStepPositive",
        segs: &["'", "'의 한 번 폭은 0보다 커야 함"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.charImgKeyTooLong",
        segs: &["이미지 키는 ", "자 이하로 정해 주세요"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.packReadConvErr",
        segs: &["대화 데이터를 읽을 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.gameMaxReadRange",
        segs: &["한 번에 ", "개까지만 읽을 수 있습니다"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.gameCharNotFoundId",
        segs: &["게임 캐릭터를 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.gameUnsupportedMime",
        segs: &["지원하지 않는 파일 형식입니다 (", ")"],
        args: &["mime"],
    },
    TemplateRule {
        key: "errors.detail.gameStoryPlanParseErr",
        segs: &["각본가 응답을 읽을 수 없습니다: ", ""],
        args: &["why"],
    },
    TemplateRule {
        key: "errors.detail.syncWriteToFolderErr",
        segs: &["동기화 폴더에 쓰지 못함 (", "): ", ""],
        args: &["name", "err"],
    },
    TemplateRule {
        key: "errors.detail.pkgZipEntryReadErr",
        segs: &["zip 항목을 읽을 수 없음 (", ")"],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.communityInvalidPluginId",
        segs: &["플러그인 id가 올바르지 않음: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.communityRateLimited",
        segs: &["호출이 너무 잦음 (초당 ", "번까지)"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.chatUnexpectedTaskOutput",
        segs: &["채팅 작업의 결과 형식이 다름: ", ""],
        args: &["other"],
    },
    TemplateRule {
        key: "errors.detail.uploadPerCallLimit",
        segs: &["한 번에 ", "개까지 올릴 수 있습니다"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.cannotReadFileContent",
        segs: &["파일 내용을 읽을 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.fileTooLargeMaxMb",
        segs: &["파일이 너무 큽니다 (최대 ", "MB)"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.packWrongCharConv",
        segs: &["고른 캐릭터의 대화가 아닙니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.stZipFinishErr",
        segs: &["zip을 마무리할 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.gameInvalidAssetPathArg",
        segs: &["에셋 경로가 올바르지 않습니다: ", ""],
        args: &["path"],
    },
    TemplateRule {
        key: "errors.detail.taskInvalidPayload",
        segs: &["작업 인자가 올바르지 않습니다: ", ""],
        args: &["detail"],
    },
    TemplateRule {
        key: "errors.detail.taskUnknownTypeDb",
        segs: &["알 수 없는 task_type: ", ""],
        args: &["val"],
    },
    TemplateRule {
        key: "errors.detail.aiParseErr",
        segs: &["", " 응답을 해석할 수 없습니다: ", ""],
        args: &["what", "err"],
    },
    TemplateRule {
        key: "errors.detail.varsDefaultNotNumber",
        segs: &["'", "'의 처음 값이 수가 아님: ", ""],
        args: &["name", "val"],
    },
    TemplateRule {
        key: "errors.detail.cannotOpenFileNamed",
        segs: &["파일을 열 수 없습니다 (", "): ", ""],
        args: &["name", "err"],
    },
    TemplateRule {
        key: "errors.detail.httpAuthFailed",
        segs: &["API 인증에 실패했습니다 (", ")"],
        args: &["status"],
    },
    TemplateRule {
        key: "errors.detail.httpRejected",
        segs: &["AI 요청이 거부되었습니다 (", ")"],
        args: &["status"],
    },
    TemplateRule {
        key: "errors.detail.vertexTokenFailed",
        segs: &["GCP 인증 토큰 발급 실패: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.unknownChatMode",
        segs: &["알 수 없는 대화 모드입니다: ", ""],
        args: &["mode"],
    },
    TemplateRule {
        key: "errors.detail.copyImageErr",
        segs: &["이미지를 복사할 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.personaNotFoundId",
        segs: &["페르소나를 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.musicFolderOpenErr",
        segs: &["음악 폴더를 열 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.cannotOpenPickedFile",
        segs: &["고른 파일을 열 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.cannotOpenSaveDest",
        segs: &["저장할 곳을 열 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.backupDbOpenErr",
        segs: &["백업 DB를 열 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.manifestMinAppVersion",
        segs: &["앱 ", " 이상이 필요함 (지금 ", ")"],
        args: &["min", "cur"],
    },
    TemplateRule {
        key: "errors.detail.archiveEntryReadErr",
        segs: &["", "을(를) 읽을 수 없습니다: ", ""],
        args: &["name", "err"],
    },
    TemplateRule {
        key: "errors.detail.pkgTooManyFiles",
        segs: &["파일이 너무 많음 (최대 ", "개)"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.communityInstallFailed",
        segs: &["플러그인을 설치할 수 없음: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.communityNotRunning",
        segs: &["플러그인이 실행 중이 아님: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingMustBeBool",
        segs: &["", ": true/false여야 함"],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.postprocessPluginNotAllowed",
        segs: &["후처리할 수 없는 플러그인: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.toolsDuplicateCallId",
        segs: &["도구 호출 id가 겹침: '", "'"],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.memoryDeleteFileErr",
        segs: &["메모리 파일을 지우지 못함: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.charDoesNotExistId",
        segs: &["존재하지 않는 캐릭터입니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.charNotFoundId",
        segs: &["캐릭터를 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.msgNotFoundFull",
        segs: &["메시지를 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.lorebookNotFound",
        segs: &["로어북을 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.presetNotFound",
        segs: &["프리셋을 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.imgReadErr",
        segs: &["이미지를 읽을 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.backupDbCorrupted",
        segs: &["백업 DB가 손상되었습니다: ", ""],
        args: &["detail"],
    },
    TemplateRule {
        key: "errors.detail.pngConvertErr",
        segs: &["PNG로 바꿀 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.gameSlotRange",
        segs: &["slot은 1~", " 이어야 합니다"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.gameChoiceNotFound",
        segs: &["존재하지 않는 선택지입니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.gameItemNotFound",
        segs: &["존재하지 않는 아이템입니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.gameEventNotFound",
        segs: &["존재하지 않는 이벤트입니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.pkgDuplicatePath",
        segs: &["같은 경로가 두 번 있음: ", ""],
        args: &["path"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingInvalidOption",
        segs: &["", ": 고를 수 있는 값이 아님"],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.convNotFoundId",
        segs: &["대화를 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.cannotCreateFile",
        segs: &["파일을 만들 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.cannotCreateDir",
        segs: &["폴더를 만들 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.cardJsonErr",
        segs: &["카드 JSON 해석 실패: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.syncHashMismatch",
        segs: &["파일 내용이 해시와 다름: ", ""],
        args: &["hash"],
    },
    TemplateRule {
        key: "errors.detail.syncBadFileName",
        segs: &["잘못된 동기화 파일 이름: ", ""],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.stZipOpenErr",
        segs: &["ZIP을 열 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.stZipWriteErr",
        segs: &["zip에 쓸 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.updateCheckErr",
        segs: &["새 버전을 확인하지 못함: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.updateReadReleaseErr",
        segs: &["릴리스 정보를 읽지 못함: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.gameNotFoundId",
        segs: &["게임을 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.gameDuplicateAssetKey",
        segs: &["이미 있는 에셋 키입니다: ", ""],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.gameUnknownTemplate",
        segs: &["알 수 없는 템플릿입니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.gameTargetNotFound",
        segs: &["존재하지 않는 대상입니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.taskUnsupportedType",
        segs: &["지원하지 않는 작업 타입: ", ""],
        args: &["taskType"],
    },
    TemplateRule {
        key: "errors.detail.taskNotFoundId",
        segs: &["작업을 찾을 수 없습니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.pkgZipOpenErr",
        segs: &["zip을 열 수 없음 (", ")"],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.pkgOpenPluginErr",
        segs: &["플러그인을 열 수 없음: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.communityNotInstalled",
        segs: &["설치된 플러그인이 아님: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.varsDuplicateName",
        segs: &["같은 이름의 변수가 둘: ", ""],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.varsMinGtMax",
        segs: &["'", "'의 최소가 최대보다 큼"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.memoryUnknownPurgeKind",
        segs: &["알 수 없는 정리 종류: ", ""],
        args: &["kind"],
    },
    TemplateRule {
        key: "errors.detail.notImageFilePath",
        segs: &["이미지 파일이 아닙니다: ", ""],
        args: &["path"],
    },
    TemplateRule {
        key: "errors.detail.musicImgLoadFailed",
        segs: &["이미지 파일 로드 실패: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.cannotOpenFile",
        segs: &["파일을 열 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.cannotWriteFile",
        segs: &["파일을 쓸 수 없습니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.syncSettingsReadErr",
        segs: &["받은 설정을 읽지 못함: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.syncFileOutsideDir",
        segs: &["동기화 폴더 밖의 파일: ", ""],
        args: &["path"],
    },
    TemplateRule {
        key: "errors.detail.driveReadRespErr",
        segs: &["구글 응답을 읽지 못함: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.stZipNotZip",
        segs: &["ZIP 파일이 아닙니다: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.gameSceneNotFoundId",
        segs: &["존재하지 않는 씬입니다: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.manifestMaxSettings",
        segs: &["settings는 ", "개까지"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.pkgSymlinkNotAllowed",
        segs: &["링크는 넣을 수 없음: ", ""],
        args: &["path"],
    },
    TemplateRule {
        key: "errors.detail.pkgMissingMainFile",
        segs: &["main 파일이 없음: ", ""],
        args: &["main"],
    },
    TemplateRule {
        key: "errors.detail.communityVersionDirMissing",
        segs: &["그런 판 폴더가 없음: ", ""],
        args: &["dir"],
    },
    TemplateRule {
        key: "errors.detail.communityVersionUnusable",
        segs: &["이 판은 쓸 수 없음: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.communityCannotEnable",
        segs: &["켤 수 없는 플러그인: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.toolsDuplicateName",
        segs: &["도구 이름이 겹침: '", "'"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.toolsMaxTail",
        segs: &["도구 호출 기록은 ", "턴까지"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.musicReadDirErr",
        segs: &["폴더를 읽는 중 오류: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.openBrowserErr",
        segs: &["브라우저를 열지 못함: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.syncNotBundleFormat",
        segs: &["동기화 묶음이 아님 (", ")"],
        args: &["format"],
    },
    TemplateRule {
        key: "errors.detail.driveConnectErr",
        segs: &["구글에 연결하지 못함: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.pkgFileReadErr",
        segs: &["", "을 읽을 수 없음 (", ")"],
        args: &["name", "err"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingTooLong",
        segs: &["", ": 너무 김 (", "자까지)"],
        args: &["key", "max"],
    },
    TemplateRule {
        key: "errors.detail.varsNameTooLong",
        segs: &["변수 이름은 ", "자까지: ", ""],
        args: &["max", "name"],
    },
    TemplateRule {
        key: "errors.detail.manifestVersionLen",
        segs: &["version은 1~", "자"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.pkgDisallowedPath",
        segs: &["허용되지 않는 경로: ", ""],
        args: &["path"],
    },
    TemplateRule {
        key: "errors.detail.pkgTooLargeMb",
        segs: &["너무 큼 (최대 ", "MB)"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.pkgMissingPanelPage",
        segs: &["패널 페이지가 없음: ", ""],
        args: &["page"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingNotDecl",
        segs: &["선언하지 않은 설정: ", ""],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingMustBeStrArr",
        segs: &["", ": 글자 배열이어야 함"],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.whatNotFound",
        segs: &["", "을(를) 찾을 수 없음"],
        args: &["what"],
    },
    TemplateRule {
        key: "errors.detail.musicOpenFileErr",
        segs: &["파일 여는 중 오류: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.musicObjCreateFailed",
        segs: &["객체 생성 중 실패: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.syncBadFileHashPath",
        segs: &["파일 해시가 이상함: ", ""],
        args: &["path"],
    },
    TemplateRule {
        key: "errors.detail.syncFolderMissing",
        segs: &["동기화 폴더가 없음: ", ""],
        args: &["path"],
    },
    TemplateRule {
        key: "errors.detail.syncMissingInFolder",
        segs: &["동기화 폴더에 없음: ", ""],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.conflictNoRollback",
        segs: &["되돌릴 사본이 없음: ", ""],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.jsonProcessErr",
        segs: &["JSON 처리 실패: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.syncRowReadErr",
        segs: &["", " 행을 읽지 못함: ", ""],
        args: &["table", "err"],
    },
    TemplateRule {
        key: "errors.detail.proposalReadonlyField",
        segs: &["고칠 수 없는 칸: ", ""],
        args: &["field"],
    },
    TemplateRule {
        key: "errors.detail.proposalTargetNotFound",
        segs: &["고칠 항목이 없음: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.proposalUnknownKind",
        segs: &["모르는 제안 종류: ", ""],
        args: &["kind"],
    },
    TemplateRule {
        key: "errors.detail.proposalAlreadyDecided",
        segs: &["이미 고른 제안 (", ")"],
        args: &["status"],
    },
    TemplateRule {
        key: "errors.detail.unknownKindArg",
        segs: &["알 수 없는 종류: ", ""],
        args: &["kind"],
    },
    TemplateRule {
        key: "errors.detail.qrUnknownScope",
        segs: &["모르는 적용 범위: ", ""],
        args: &["scope"],
    },
    TemplateRule {
        key: "errors.detail.musicTaskExecErr",
        segs: &["태스크 실행 오류: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.invalidPickedUri",
        segs: &["잘못된 파일 주소: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.trashDeleteMissing",
        segs: &["지울 항목이 없음: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.storageKeyLen",
        segs: &["저장소 키는 1~", "자"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.cannotReadErr",
        segs: &["읽을 수 없음 (", ")"],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.proposalParentNotFound",
        segs: &["넣을 곳이 없음: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.varsMaxDefs",
        segs: &["변수 정의는 ", "개까지"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.musicSaveFileFailed",
        segs: &["파일 저장 실패: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.watcherRecvErr",
        segs: &["채널 수신 오류: ", ""],
        args: &["err"],
    },
    TemplateRule {
        key: "errors.detail.syncRowInvalidShape",
        segs: &["", " 행 모양이 이상함"],
        args: &["table"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingItemMaxChars",
        segs: &["", ": 항목은 ", "자까지"],
        args: &["key", "max"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingUnknownKind",
        segs: &["", ": 모르는 종류 ", ""],
        args: &["key", "kind"],
    },
    TemplateRule {
        key: "errors.detail.communityDataNotFound",
        segs: &["", "이(가) 없음: ", ""],
        args: &["what", "id"],
    },
    TemplateRule {
        key: "errors.detail.pluginPackagePrefix",
        segs: &["플러그인 묶음: ", ""],
        args: &["msg"],
    },
    TemplateRule {
        key: "errors.detail.manifestNameLen",
        segs: &["name은 1~", "자"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.communitySuspended",
        segs: &["멈춘 플러그인: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.memoryQueryTooLong",
        segs: &["찾을 말은 ", "자까지"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.msgNotFoundColon",
        segs: &["메시지가 없음: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.archiveEntryMissing",
        segs: &["", "이(가) 없습니다"],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.syncTableRowReadErr",
        segs: &["", " 행을 읽지 못함"],
        args: &["table"],
    },
    TemplateRule {
        key: "errors.detail.conflictUnknownTable",
        segs: &["모르는 테이블: ", ""],
        args: &["table"],
    },
    TemplateRule {
        key: "errors.detail.manifestUnknownPerm",
        segs: &["모르는 권한: ", ""],
        args: &["perm"],
    },
    TemplateRule {
        key: "errors.detail.communityUnknownMethod",
        segs: &["없는 메서드: ", ""],
        args: &["method"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingMustBeStr",
        segs: &["", ": 글이어야 함"],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingMustBeChar",
        segs: &["", ": 글자여야 함"],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingMustBeNum",
        segs: &["", ": 숫자여야 함"],
        args: &["key"],
    },
    TemplateRule {
        key: "errors.detail.proposalUnknownOp",
        segs: &["모르는 op: ", ""],
        args: &["op"],
    },
    TemplateRule {
        key: "errors.detail.proposalNotFound",
        segs: &["제안이 없음: ", ""],
        args: &["id"],
    },
    TemplateRule {
        key: "errors.detail.unknownFormatArg",
        segs: &["모르는 형식: ", ""],
        args: &["format"],
    },
    TemplateRule {
        key: "errors.detail.archiveUnsafePathNamed",
        segs: &["잘못된 경로: ", ""],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.toolsMaxLimit",
        segs: &["도구는 ", "개까지"],
        args: &["max"],
    },
    TemplateRule {
        key: "errors.detail.conflictBackupMissing",
        segs: &["없는 사본: ", ""],
        args: &["name"],
    },
    TemplateRule {
        key: "errors.detail.syncRowMissing",
        segs: &["", " 행이 없음"],
        args: &["table"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingMaxChars",
        segs: &["", ": ", "자까지"],
        args: &["key", "max"],
    },
    TemplateRule {
        key: "errors.detail.communitySettingMaxItems",
        segs: &["", ": ", "개까지"],
        args: &["key", "max"],
    },
    TemplateRule {
        key: "errors.detail.communityDataKeyRequired",
        segs: &["", "가 필요함"],
        args: &["key"],
    },
];

fn match_template(rule: &TemplateRule, text: &str) -> Option<BTreeMap<String, String>> {
    let segs = rule.segs;
    let args = rule.args;
    let first = segs.first()?;
    let last = segs.last()?;
    let min_len: usize = segs.iter().map(|s| s.len()).sum();
    if text.len() < min_len || !text.starts_with(first) || !text.ends_with(last) {
        return None;
    }
    let mut pos = first.len();
    let end_limit = text.len() - last.len();
    let mut out = BTreeMap::new();
    for (i, arg_name) in args.iter().enumerate() {
        if i + 1 == args.len() {
            out.insert((*arg_name).to_string(), text[pos..end_limit].to_string());
        } else {
            let next_seg = segs[i + 1];
            let rel = text[pos..end_limit].find(next_seg)?;
            let idx = pos + rel;
            out.insert((*arg_name).to_string(), text[pos..idx].to_string());
            pos = idx + next_seg.len();
        }
    }
    Some(out)
}

/// 한국어 오류 문구에서 `(key, args)`를 추출한다.
pub fn classify_message(message: &str) -> Option<ErrorI18n> {
    let msg = message.trim();
    if msg.is_empty() {
        return None;
    }

    // 1. 정확히 일치하는 고정 문구
    for (pat, key) in EXACT_RULES {
        if *pat == msg {
            return Some(ErrorI18n {
                key: (*key).to_string(),
                args: None,
            });
        }
    }

    // 2. 래퍼 접두사(manifest / 묶음 / 각본가 등)는 안쪽 사유도 재귀 분류해 subKey로 첨부
    for (prefix, wrap_key) in [
        ("플러그인 manifest.json: ", "errors.detail.pluginManifestPrefix"),
        ("플러그인 묶음: ", "errors.detail.pluginPackagePrefix"),
    ] {
        if let Some(inner) = msg.strip_prefix(prefix) {
            let mut args = BTreeMap::new();
            args.insert("msg".to_string(), inner.to_string());
            if let Some(sub) = classify_message(inner) {
                args.insert("subKey".to_string(), sub.key);
                if let Some(sub_args) = sub.args {
                    for (k, v) in sub_args {
                        args.entry(k).or_insert(v);
                    }
                }
            }
            return Some(ErrorI18n {
                key: wrap_key.to_string(),
                args: Some(args),
            });
        }
    }

    // 3. 인자 템플릿 매칭 (긴 리터럴 우선 정렬됨)
    for rule in TEMPLATE_RULES {
        if let Some(mut args) = match_template(rule, msg) {
            // 안쪽 사유(`why` / `reason`)가 다시 분류 가능하면 subKey 첨부
            for inner_field in ["why", "reason"] {
                if let Some(inner_val) = args.get(inner_field).cloned() {
                    if let Some(sub) = classify_message(&inner_val) {
                        args.insert(format!("subKey_{inner_field}"), sub.key);
                        if let Some(sub_args) = sub.args {
                            for (k, v) in sub_args {
                                args.insert(format!("sub_{inner_field}_{k}"), v);
                            }
                        }
                    }
                }
            }
            return Some(ErrorI18n {
                key: rule.key.to_string(),
                args: if args.is_empty() { None } else { Some(args) },
            });
        }
    }

    None
}

/// `AppError`의 코드·메시지·details를 함께 보고 다국어 키와 인자를 결정한다.
pub fn classify(code: ErrorCode, message: &str, details: Option<&Value>) -> Option<ErrorI18n> {
    if let Some(plugin) = details.and_then(|d| d.get("plugin")).and_then(Value::as_str) {
        if !plugin.is_empty() {
            let mut args = BTreeMap::new();
            args.insert("plugin".to_string(), plugin.to_string());
            return Some(ErrorI18n {
                key: "errors.pluginDisabled".to_string(),
                args: Some(args),
            });
        }
    }

    if let Some(found) = classify_message(message) {
        return Some(found);
    }

    if code == ErrorCode::GameActionBlocked {
        if let Some(reason) = details.and_then(|d| d.get("reason")).and_then(Value::as_str) {
            if !reason.is_empty() {
                return Some(ErrorI18n {
                    key: format!("errors.gameBlocked.{reason}"),
                    args: None,
                });
            }
        }
    }

    None
}

/// 문자열 에러 코드(`TaskErrorInfo`)에서 다국어 키·인자를 결정한다.
pub fn classify_code_str(_code: &str, message: &str) -> Option<ErrorI18n> {
    classify_message(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn classifies_parameterized_and_wrapped_errors() {
        let r = classify_message("캐릭터를 찾을 수 없습니다: abc-123").expect("matched");
        assert_eq!(r.key, "errors.detail.charNotFoundId");
        assert_eq!(r.args.as_ref().and_then(|m| m.get("id")).map(String::as_str), Some("abc-123"));

        let w = classify_message("플러그인 manifest.json: id는 소문자·숫자·'-'로 2~40자여야 함: \"Bad\"").expect("wrapped");
        assert_eq!(w.key, "errors.detail.pluginManifestPrefix");
        let args = w.args.expect("args");
        assert_eq!(args.get("subKey").map(String::as_str), Some("errors.detail.manifestInvalidId"));
        assert_eq!(args.get("max").map(String::as_str), Some("40"));
        assert_eq!(args.get("id").map(String::as_str), Some("\"Bad\""));
    }

    #[test]
    fn every_rule_key_exists_in_ko_and_en_backend_json_without_hangul_leak() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("repo root");
        let ko: Value = serde_json::from_str(&std::fs::read_to_string(root.join("src/locales/ko/backend.json")).expect("ko/backend.json")).expect("valid json");
        let en: Value = serde_json::from_str(&std::fs::read_to_string(root.join("src/locales/en/backend.json")).expect("en/backend.json")).expect("valid json");

        let lookup = |doc: &Value, dotted: &str| -> Option<String> {
            let mut cur = doc;
            for part in dotted.split('.') {
                cur = cur.get(part)?;
            }
            cur.as_str().map(str::to_string)
        };

        let all_keys = EXACT_RULES.iter().map(|(_, k)| *k).chain(TEMPLATE_RULES.iter().map(|r| r.key));

        for key in all_keys {
            let ko_val = lookup(&ko, key).unwrap_or_else(|| panic!("missing in ko/backend.json: {key}"));
            let en_val = lookup(&en, key).unwrap_or_else(|| panic!("missing in en/backend.json: {key}"));
            assert!(!ko_val.is_empty(), "empty ko value for {key}");
            assert!(!en_val.is_empty(), "empty en value for {key}");
            assert!(
                !en_val.chars().any(|c| ('\u{AC00}'..='\u{D7A3}').contains(&c)),
                "Hangul leaked in en/backend.json for {key}: {en_val}"
            );
        }
    }
}
