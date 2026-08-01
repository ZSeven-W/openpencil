//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `ko_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "이미지 검색…",
        "imagePanel.searching" => "검색 중…",
        "imagePanel.noResults" => "결과가 없습니다",
        "imagePanel.searchPrompt" => "이미지를 검색하세요",
        "imagePanel.sourceNotice" => {
            "{{source}} 제공 이미지. 자유 라이선스 — 사용 전에 라이선스를 확인하세요."
        }
        "imagePanel.genNotConfigured" => "이미지 생성이 설정되지 않았습니다",
        "imagePanel.openSettings" => "설정 열기",
        "imagePanel.promptPlaceholder" => "이미지를 설명하세요…",
        "providerProbe.connectedViaCli" => "{{name}} CLI를 통해 연결됨",
        "providerProbe.cliExitedWithError" => "{{name}} CLI가 오류와 함께 종료되었습니다",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI가 버전 정보를 출력하지 않았습니다",
        "providerProbe.modelQueryFailed" => "{{name}} 모델 조회에 실패했거나 시간이 초과되었습니다",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} 모델 조회에 실패했습니다. {{command}}을(를) 한 번 실행해 인증하세요."
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} 모델 조회에는 인증이 필요합니다. {{command}}을(를) 한 번 실행해 로그인하세요."
        }
        "providerProbe.unrecognizedModelCatalog" => {
            "{{name}}이(가) 인식할 수 없는 모델 목록을 반환했습니다"
        }
        "promptCenter.title" => "프롬프트 센터",
        "promptCenter.searchPlaceholder" => "프롬프트 검색…",
        "promptCenter.category.all" => "전체",
        "promptCenter.category.starter" => "빠른 시작",
        "promptCenter.category.mobileApp" => "모바일 앱",
        "promptCenter.category.webPage" => "웹 페이지",
        "promptCenter.category.dashboard" => "대시보드",
        "promptCenter.category.component" => "컴포넌트",
        "promptCenter.category.modify" => "수정",
        "promptCenter.category.custom" => "나의 프롬프트",
        "promptCenter.empty" => "일치하는 프롬프트가 없습니다",
        "promptCenter.saveCurrent" => "현재 입력 저장",
        "promptCenter.saveTitlePlaceholder" => "프롬프트 제목",
        "promptCenter.save" => "저장",
        "promptCenter.cancel" => "취소",
        "promptCenter.delete" => "삭제",
        "promptCenter.screens" => "{{count}}개 화면",
        "promptCenter.freeform" => "자유 형식",
        "promptCenter.item.wander.title" => "Wander · 여행 일정",
        "promptCenter.item.forage.title" => "Forage · 제철 레시피",
        "promptCenter.item.still.title" => "Still · 명상과 수면",
        "promptCenter.item.hearth.title" => "Hearth · 스마트 홈",
        "promptCenter.item.meteo.title" => "Meteo · 몰입형 날씨",
        "promptCenter.item.marginalia.title" => "Marginalia · 독서와 주석",
        "promptCenter.item.lingua.title" => "Lingua · 언어 학습",
        "promptCenter.item.daybreak.title" => "Daybreak · 커피 주문",
        "promptCenter.item.verdant.title" => "Verdant · 식물 관리",
        "promptCenter.item.companion.title" => "Companion · 반려동물 생활",
        "promptCenter.item.relic.title" => "Relic · 엄선 중고 마켓",
        "promptCenter.item.nocturne.title" => "Nocturne · 별 관측 가이드",
        "promptCenter.item.marquee.title" => "Marquee · 영화 감상 목록",
        "promptCenter.item.ritual.title" => "Ritual · 습관 만들기",
        "promptCenter.item.ember.title" => "Ember · 감정 일기",
        "promptCenter.item.volt.title" => "Volt · 전기차 동반자",
        "promptCenter.item.aloft.title" => "Aloft · 항공편 추적",
        "promptCenter.item.gallery.title" => "Gallery · 전시와 문화 행사",
        "promptCenter.item.nightcap.title" => "Nightcap · 홈 칵테일",
        "promptCenter.item.bloom.title" => "Bloom · 아이 성장 기록",
        "promptCenter.item.extremeWeather.title" => "극한 · 날씨 앱",
        "promptCenter.item.extremeNowPlaying.title" => "극한 · 지금 재생",
        "promptCenter.item.extremeDailyApp.title" => "극한 · 매일 쓰고 싶은 앱",
        "promptCenter.item.extremeCalendar.title" => "극한 · 캘린더",
        "promptCenter.item.extremeCalm.title" => "극한 · 평온",
        "promptCenter.item.webOrbit.title" => "Orbit · AI 워크벤치 랜딩 페이지",
        "promptCenter.item.webAtelier.title" => "Atelier · 가구 브랜드 커머스",
        "promptCenter.item.dashboardPulse.title" => "Pulse · 성장 분석 대시보드",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · 물류 운영",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · 엔터프라이즈 데이터 테이블",
        "promptCenter.item.componentFormLab.title" => "Form Lab · 폼 컴포넌트 시스템",
        "promptCenter.item.modifyPolishCurrent.title" => "현재 화면 다듬기",
        "promptCenter.item.modifyCompleteStates.title" => "컴포넌트 상태 완성하기",
        "collab.ownerConfirm.title" => "참여할 상대를 확인하세요",
        "collab.ownerConfirm.hint" => "이 세션의 내용은 아직 아무것도 불러오지 않았습니다.",
        "collab.ownerConfirm.account" => "검증된 계정",
        "collab.ownerConfirm.device" => "검증된 기기",
        "collab.ownerConfirm.claimedName" => "이 계정이 정한 이름(검증되지 않음)",
        "collab.action.confirmOwner" => "이 세션에 참여",
        "collab.action.rejectOwner" => "참여 안 함",
        "collab.error.ownerNotConfirmed" => "호스트를 확인하지 않아 아무것도 불러오지 않았습니다.",
        "sceneTemplate.title" => "장면 템플릿",
        "sceneTemplate.searchPlaceholder" => "장면 또는 템플릿 검색…",
        "sceneTemplate.empty" => "일치하는 템플릿이 없습니다",
        "sceneTemplate.frames" => "{{count}}페이지",
        "sceneTemplate.filter.all" => "전체",
        "sceneTemplate.scene.tutorial" => "튜토리얼 이미지",
        "sceneTemplate.scene.comparison" => "비교 이미지",
        "sceneTemplate.scene.carousel" => "지식 카드",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.item.screenshotTutorial.title" => "3단계 스크린샷 튜토리얼 카드",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "표지, 3단계 작업, 마지막 CTA로 구성되어 있어 스크린샷과 설명만 바꾸면 바로 게시할 수 있습니다."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "지식·인사이트 캐러셀",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "표지, 세 가지 논점, 요약 페이지로 구성되어 있어 하나의 관점을 넘겨 보는 연속 카드로 풀어내기에 적합합니다."
        }
        "sceneTemplate.item.beforeAfter.title" => "개편 전후 비교",
        "sceneTemplate.item.beforeAfter.summary" => {
            "전후 화면을 좌우로 나란히 배치하고 변경 설명을 더해, 회고나 작업물 소개에 적합합니다."
        }
        "sceneTemplate.item.slideDeck.title" => "프레젠테이션 · 6페이지",
        "sceneTemplate.item.slideDeck.summary" => {
            "표지, 목차, 핵심 내용, 데이터, 차트, 마무리로 구성된 16:9 발표 자료로, 문구만 바꾸면 바로 발표할 수 있습니다."
        }
        "fileMenu.newFromTemplate" => "템플릿에서 새로 만들기",
        _ => return super::ko_collab::lookup(key),
    })
}
