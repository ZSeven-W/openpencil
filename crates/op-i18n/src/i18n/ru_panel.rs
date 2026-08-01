//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `ru_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Поиск изображений…",
        "imagePanel.searching" => "Поиск…",
        "imagePanel.noResults" => "Ничего не найдено",
        "imagePanel.searchPrompt" => "Найдите изображения",
        "imagePanel.sourceNotice" => "Изображения из {{source}}. Свободная лицензия — проверьте лицензию перед использованием.",
        "imagePanel.genNotConfigured" => "Генерация изображений не настроена",
        "imagePanel.openSettings" => "Открыть настройки",
        "imagePanel.promptPlaceholder" => "Опишите изображение…",
        "providerProbe.connectedViaCli" => "Подключено через CLI {{name}}",
        "providerProbe.cliExitedWithError" => "CLI {{name}} завершилась с ошибкой",
        "providerProbe.cliNoVersionOutput" => "CLI {{name}} не вернула сведения о версии",
        "providerProbe.modelQueryFailed" => "Запрос моделей {{name}} завершился ошибкой или тайм-аутом",
        "providerProbe.modelQueryFailedRunLogin" => "Запрос моделей {{name}} завершился ошибкой. Выполните {{command}} один раз для аутентификации.",
        "providerProbe.modelQueryNeedsAuth" => "Для запроса моделей {{name}} требуется аутентификация. Выполните {{command}} один раз, чтобы войти.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} вернул нераспознанный каталог моделей",
        "promptCenter.title" => "Библиотека промптов",
        "promptCenter.searchPlaceholder" => "Поиск промптов…",
        "promptCenter.category.all" => "Все",
        "promptCenter.category.starter" => "Быстрый старт",
        "promptCenter.category.mobileApp" => "Мобильное приложение",
        "promptCenter.category.webPage" => "Веб-страница",
        "promptCenter.category.dashboard" => "Дашборд",
        "promptCenter.category.component" => "Компонент",
        "promptCenter.category.modify" => "Редактирование",
        "promptCenter.category.custom" => "Мои",
        "promptCenter.empty" => "Подходящих промптов не найдено",
        "promptCenter.saveCurrent" => "Сохранить текущий текст как промпт",
        "promptCenter.saveTitlePlaceholder" => "Название промпта",
        "promptCenter.save" => "Сохранить",
        "promptCenter.cancel" => "Отмена",
        "promptCenter.delete" => "Удалить",
        "promptCenter.screens" => "{{count}} экранов",
        "promptCenter.freeform" => "Свободная форма",
        "promptCenter.item.wander.title" => "Wander · Планирование путешествий",
        "promptCenter.item.forage.title" => "Forage · Сезонные рецепты",
        "promptCenter.item.still.title" => "Still · Медитация и сон",
        "promptCenter.item.hearth.title" => "Hearth · Умный дом",
        "promptCenter.item.meteo.title" => "Meteo · Иммерсивная погода",
        "promptCenter.item.marginalia.title" => "Marginalia · Чтение и заметки",
        "promptCenter.item.lingua.title" => "Lingua · Изучение языков",
        "promptCenter.item.daybreak.title" => "Daybreak · Заказ кофе",
        "promptCenter.item.verdant.title" => "Verdant · Уход за растениями",
        "promptCenter.item.companion.title" => "Companion · Жизнь с питомцами",
        "promptCenter.item.relic.title" => "Relic · Отборный секонд-хенд маркет",
        "promptCenter.item.nocturne.title" => "Nocturne · Путеводитель по звёздному небу",
        "promptCenter.item.marquee.title" => "Marquee · Список фильмов",
        "promptCenter.item.ritual.title" => "Ritual · Формирование привычек",
        "promptCenter.item.ember.title" => "Ember · Дневник настроения",
        "promptCenter.item.volt.title" => "Volt · Помощник для электромобиля",
        "promptCenter.item.aloft.title" => "Aloft · Отслеживание рейсов",
        "promptCenter.item.gallery.title" => "Gallery · Выставки и культура",
        "promptCenter.item.nightcap.title" => "Nightcap · Домашние коктейли",
        "promptCenter.item.bloom.title" => "Bloom · Семейный дневник развития",
        "promptCenter.item.extremeWeather.title" => "Экстрим · Погодное приложение",
        "promptCenter.item.extremeNowPlaying.title" => "Экстрим · Сейчас играет",
        "promptCenter.item.extremeDailyApp.title" => "Экстрим · Открывать каждый день",
        "promptCenter.item.extremeCalendar.title" => "Экстрим · Переизобрести календарь",
        "promptCenter.item.extremeCalm.title" => "Экстрим · Один экран спокойствия",
        "promptCenter.item.webOrbit.title" => "Orbit · Лендинг ИИ-рабочего пространства",
        "promptCenter.item.webAtelier.title" => "Atelier · Интернет-магазин мебели",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Панель аналитики роста",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Логистические операции",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Корпоративная таблица данных",
        "promptCenter.item.componentFormLab.title" => "Form Lab · Система компонентов форм",
        "promptCenter.item.modifyPolishCurrent.title" => "Улучшить текущий экран",
        "promptCenter.item.modifyCompleteStates.title" => "Дополнить состояния компонентов",
        "collab.ownerConfirm.title" => "Подтвердите, к кому вы присоединяетесь",
        "collab.ownerConfirm.hint" => "Из этого сеанса пока ничего не загружено.",
        "collab.ownerConfirm.account" => "Подтверждённый аккаунт",
        "collab.ownerConfirm.device" => "Подтверждённое устройство",
        "collab.ownerConfirm.claimedName" => "Имя, выбранное этим аккаунтом (не подтверждено)",
        "collab.action.confirmOwner" => "Присоединиться к сеансу",
        "collab.action.rejectOwner" => "Не присоединяться",
        "collab.error.ownerNotConfirmed" => "Вы не подтвердили ведущего, поэтому ничего не загружено.",
        "sceneTemplate.title" => "Шаблоны сцен",
        "sceneTemplate.searchPlaceholder" => "Поиск сцен и шаблонов…",
        "sceneTemplate.empty" => "Подходящих шаблонов не найдено",
        "sceneTemplate.frames" => "Страницы: {{count}}",
        "sceneTemplate.filter.all" => "Все",
        "sceneTemplate.scene.tutorial" => "Обучающая графика",
        "sceneTemplate.scene.comparison" => "Сравнительная графика",
        "sceneTemplate.scene.carousel" => "Карточки знаний",
        "sceneTemplate.scene.slides" => "Презентация",
        "sceneTemplate.item.screenshotTutorial.title" => "Скриншот-инструкция · 3 шага",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Обложка, три шага и финальный призыв к действию. Замените скриншоты и текст — и можно публиковать."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Карусель знаний и идей",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Обложка, три тезиса и итоговый слайд — удобно разложить одну идею на последовательные карточки для свайпа."
        }
        "sceneTemplate.item.beforeAfter.title" => "Сравнение редизайна «до/после»",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Версии до и после рядом с пояснениями изменений — для ретроспектив и портфолио."
        }
        "sceneTemplate.item.slideDeck.title" => "Презентация · 6 слайдов",
        "sceneTemplate.item.slideDeck.summary" => {
            "Обложка, повестка, ключевые тезисы, данные, диаграмма и финал в формате 16:9. Замените текст — и можно выступать."
        }
        "fileMenu.newFromTemplate" => "Создать из шаблона",
        _ => return super::ru_collab::lookup(key),
    })
}
