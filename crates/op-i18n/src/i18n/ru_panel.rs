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
        "promptCenter.item.webKilnform.title" => "Kilnform · Сайт дизайн-инфраструктуры",
        "promptCenter.item.webReefwright.title" => "Reefwright · Сайт базы знаний ИИ-поддержки",
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
        "sceneTemplate.generate.placeholder" => "Опишите тему — ИИ создаст всю презентацию",
        "sceneTemplate.generate.button" => "Создать",
        "sceneTemplate.generate.hint" => "Новый документ: тема превращается в готовую презентацию.",
        "sceneTemplate.generate.promptTemplate" => "Создай презентацию (PPT) на следующую тему: {{topic}}",
        "sceneTemplate.card.addToCanvas" => "Добавить на холст",
        "sceneTemplate.card.generateFrom" => "Создать в этом стиле",
        "sceneTemplate.generate.basis" => "На основе: ",
        "sceneTemplate.filter.all" => "Все",
        "sceneTemplate.scene.tutorial" => "Уроки",
        "sceneTemplate.scene.comparison" => "Сравнение",
        "sceneTemplate.scene.carousel" => "Карусель",
        "sceneTemplate.scene.slides" => "Слайды",
        "sceneTemplate.scene.card" => "Карточки",
        "sceneTemplate.scene.web" => "Веб-страницы",
        "sceneTemplate.generate.webPromptTemplate" => "Спроектируй многосекционную веб-страницу (лендинг) по следующей теме: {{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "SaaS-лендинг · Оранжевый",
        "sceneTemplate.item.saasLandingOrange.summary" => "Светлая маркетинговая страница на почти чёрных панелях с одним оранжевым: навигация, первый экран со скриншотом продукта, три карточки возможностей, разбор рабочего процесса, отзывы и подписка в подвале. Замените тексты — и это готовый сайт.",
        "sceneTemplate.item.productLandingLight.title" => "Страница продукта · Светлая",
        "sceneTemplate.item.productLandingLight.summary" => "Бумажно-белая продуктовая страница в газетной подаче: интерактивное демо на первом экране, колонки возможностей, панель аналитики, сравнение «до и после» и три тарифа. Для SaaS-сайтов и запусков.",
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
        "sceneTemplate.item.knowledgeCardVertical.title" => "Карточка знаний · Вертикальная",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "Одна карточка 3:4 с заголовком, четырьмя тезисами и подписью. Замените текст — и можно публиковать.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "Карточка знаний · Квадратная",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "Карточка 1:1 в той же вёрстке, компактная для обложки записи или поста в соцсети.",
        "sceneTemplate.item.pitchDeckDark.title" => "Питч-дек · Тёмный",
        "sceneTemplate.item.pitchDeckDark.summary" => "Обложка, проблема, решение, цифры, дорожная карта и контакты. Крупный шрифт на тёмном фоне — для инвестраундов и запусков.",
        "sceneTemplate.item.lectureDeckLight.title" => "Лекционные слайды · Светлый",
        "sceneTemplate.item.lectureDeckLight.summary" => "Титул курса, цели, разбор понятия, решённый пример, таблица сравнения и итог с домашним заданием. Бумажно-белый фон, не устают глаза.",
        "sceneTemplate.item.minimalKeynote.title" => "Минималистичный keynote",
        "sceneTemplate.item.minimalKeynote.summary" => "Много воздуха, огромный кегль, одна фраза по центру полосы — девять страниц без единой карточки, а оглавление — только линейки и цифры. Для запусков и докладов.",
        "sceneTemplate.item.gradientTech.title" => "Градиентный tech",
        "sceneTemplate.item.gradientTech.summary" => "Тёмный градиент и карточки из матового стекла: архитектура, замеры производительности и стена клиентов. Для запуска продукта для разработчиков.",
        "sceneTemplate.scene.infographic" => "Инфографика",
        "sceneTemplate.item.punchQuoteCard.title" => "Карточка-цитата · Плакат",
        "sceneTemplate.item.punchQuoteCard.summary" => "Карточка 3:4 на почти чёрном фоне: две огромные строки над жёлтой полосой. Одна фраза и ничего лишнего — для мнений и цитат.",
        "sceneTemplate.item.journalChecklistCard.title" => "Карточка-чеклист · База знаний",
        "sceneTemplate.item.journalChecklistCard.summary" => "Белая карточка на светло-сером фоне: пять задач с галочками, метка и блок цитаты. Для планов на неделю.",
        "sceneTemplate.item.dataReportInfographic.title" => "Инфографика с выводами",
        "sceneTemplate.item.dataReportInfographic.summary" => "Высокая картинка с прокруткой: тёмная шапка, три крупных числа, сравнение полосами, доли и три вывода. Замените цифры — и можно публиковать.",
        "sceneTemplate.item.stepsFlowInfographic.title" => "Инфографика по шагам",
        "sceneTemplate.item.stepsFlowInfographic.summary" => "Высокая картинка с прокруткой: пять пронумерованных шагов в один поток, у каждого своя длительность, плюс два совета. Для инструкций и гайдов.",
        "sceneTemplate.item.eventPosterDeck.title" => "Дек мероприятия · Афиша",
        "sceneTemplate.item.eventPosterDeck.summary" => "Обложка, что интересного, программа, как добраться, билеты и финал. Галерейно-белый фон, красные и синие плашки, без скруглений и без градиентов — для маркетов, клубных встреч и открытий.",
        "sceneTemplate.item.pitfallListInfographic.title" => "Инфографика с типичными ошибками",
        "sceneTemplate.item.pitfallListInfographic.summary" => "Высокая картинка с прокруткой: шесть ошибок по частоте, у каждой — что не так и как сделать иначе, плюс проверка из четырёх пунктов перед публикацией. Только чёрный, белый и серый.",
        "sceneTemplate.item.spineCultureCard.title" => {
            "Карточка с вертикальным заголовком · Минеральные пигменты"
        }
        "sceneTemplate.item.spineCultureCard.summary" => "Карточка 3:4 на тёмной охристой глине: вертикальный китайский заголовок, осыпающаяся штукатурка, крупицы пигмента. Для культуры, лонгридов и авторских обложек.",
        "sceneTemplate.item.metricSingleCard.title" => "Карточка одного числа · Сетка и иероглифы",
        "sceneTemplate.item.metricSingleCard.summary" => "Карточка 1:1: одно огромное число на чистом белом, строгая швейцарская сетка и один красный квадрат-сигнал. Для выводов и результатов.",
        "sceneTemplate.item.quoteFrameCard.title" => "Карточка цитаты · Шёлк сине-зелёный",
        "sceneTemplate.item.quoteFrameCard.summary" => "Карточка 4:5 на пожелтевшем шёлке: одна фраза в рамке, у нижнего края — горы из азурита и малахита. Для выдержек, интервью и цитат.",
        "sceneTemplate.item.dailySignCard.title" => "Карточка дня · Садовое окно",
        "sceneTemplate.item.dailySignCard.summary" => "Карточка 3:4 на извёстковой стене с шестиугольным окном: внутри дата и одна строка. Пустота и есть украшение. Для ежедневных постов.",
        "sceneTemplate.item.priceTierCard.title" => "Карточка тарифов · Неон галереи",
        "sceneTemplate.item.priceTierCard.summary" => "Карточка 1:1 на чернильно-синей ночи: три тарифа, контуры неоновых трубок и их свечение. Для магазинов, событий и пакетов.",
        "sceneTemplate.item.noticeBoardCard.title" => "Карточка объявления · Свинцовый набор",
        "sceneTemplate.item.noticeBoardCard.summary" => "Карточка 4:5 на газетной бумаге: линейки шапки со сбитой приводкой, пронумерованные пункты и серийный штамп. Для объявлений и правил.",
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "Инфографика-хронология",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "Высокая картинка с прокруткой: одна ось на всю высоту, отметки лет рядом с карточками вех, в конце — следующий шаг. Для итогов, истории бренда и хода проекта.",
        "sceneTemplate.item.conceptContrastInfographic.title" => "Инфографика сравнения понятий",
        "sceneTemplate.item.conceptContrastInfographic.summary" => "Высокая картинка с прокруткой: сначала вывод, затем по карточке-определению на каждое понятие, разбор по критериям в две колонки и в конце — как выбирать.",
        "sceneTemplate.item.rankingBoardInfographic.title" => "Инфографика рейтинга Топ-N",
        "sceneTemplate.item.rankingBoardInfographic.summary" => "Высокая картинка с прокруткой: золото по чернильному фону — крупные значки у первой тройки, контурные с четвёртого по восьмое, у каждого — когда и как часто применять.",
        "sceneTemplate.item.faqThreadInfographic.title" => "Инфографика вопросов и ответов",
        "sceneTemplate.item.faqThreadInfographic.summary" => "Высокая картинка с прокруткой: шесть пар «вопрос — ответ», В залит, О контурный. Ни нумерации, ни порядка: любая пара работает сама по себе.",
        "sceneTemplate.item.dataStoryInfographic.title" => "Инфографика истории в данных",
        "sceneTemplate.item.dataStoryInfographic.summary" => "Высокая картинка с прокруткой: четыре числа, связанные в одну причинную цепочку, каждый шаг — сетка из десяти клеток, в конце — вывод, который можно применить.",
        "sceneTemplate.item.challengeTrackerInfographic.title" => {
            "Инфографика 30-дневного челленджа"
        }
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "Высокая картинка с прокруткой: сетка из тридцати клеток, шесть на пять, вехи только на 7, 15 и 30 день. Сохраните и вычёркивайте по клетке в день.",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "Инфографика карты рынка",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => "Высокая картинка с прокруткой: четыре звена одной цепочки в сетке два на два, по три игрока в каждом, пустоты отмечены. Белые карточки на сланцевом фоне.",
        "sceneTemplate.item.doDontComparison.title" => "Две колонки: как надо и как не надо",
        "sceneTemplate.item.doDontComparison.summary" => "Карточка 3:4: два способа сделать одно и то же рядом, различаются фактурой и значком, а не красным и зелёным — читается и при дальтонизме.",
        "sceneTemplate.item.mythTruthComparison.title" => "Мифы и правда",
        "sceneTemplate.item.mythTruthComparison.summary" => "Высокая картинка: пять пар «говорят так / на самом деле», миф уже и светлее слева, правда шире и темнее справа.",
        "sceneTemplate.item.pricingTiersComparison.title" => "Сравнение тарифов",
        "sceneTemplate.item.pricingTiersComparison.summary" => "Карточка 3:4: бесплатный, Pro и командный рядом, цена — точка отсчёта, каждый столбец включает предыдущий.",
        "sceneTemplate.item.scenarioGuideComparison.title" => "Подбор по ситуации",
        "sceneTemplate.item.scenarioGuideComparison.summary" => "Высокая картинка: без характеристик — семь ситуаций, у каждой свой вердикт. Читателю нужно найти свою строку.",
        "sceneTemplate.item.specTableComparison.title" => "Таблица характеристик",
        "sceneTemplate.item.specTableComparison.summary" => "Высокая картинка: два кандидата в одной таблице, строка за строкой, победившая ячейка поднята тёмным фоном.",
        "sceneTemplate.item.threeWayComparison.title" => "Сравнение трёх вариантов",
        "sceneTemplate.item.threeWayComparison.summary" => "Высокая картинка: три варианта рядом, рекомендуемый — в середине; каждый столбец начинается с ситуации, а не с названия.",
        "sceneTemplate.item.timeShiftComparison.title" => "Год назад и сейчас",
        "sceneTemplate.item.timeShiftComparison.summary" => "Карточка 3:4: центральный стержень подписей, год назад слева, сейчас справа, оба значения на одной строке.",
        "sceneTemplate.item.tradeoffScaleComparison.title" => "Весы плюсов и минусов",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => "Карточка 1:1: коромысло и две чаши — слева что получите, справа чем заплатите, перед каждой строкой пустой квадратик.",
        "sceneTemplate.item.versionDiffComparison.title" => "Что изменилось в версии",
        "sceneTemplate.item.versionDiffComparison.summary" => {
            "Карточка 1:1: без деления на колонки — каждая строка сама завершает «было → стало»."
        }
        "sceneTemplate.item.appOnboardingTriptych.title" => "Триптих онбординга приложения",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "Карточка 3:4: три телефона в ряд с пустыми местами под картинки. Вставьте свои три экрана, добавьте текст — и можно на ревью или в ленту.",
        "sceneTemplate.item.diyBlueprintGuide.title" => "Иллюстрированное руководство DIY",
        "sceneTemplate.item.diyBlueprintGuide.summary" => "Высокая картинка, где таблица материалов занимает столько же места, сколько шаги: в самоделках ошибаются на подготовке, а не руками.",
        "sceneTemplate.item.photoCompositionTutorial.title" => "Композиция в съёмке на телефон",
        "sceneTemplate.item.photoCompositionTutorial.summary" => "3:4, пять кадров: тёмный видоискатель и яркие направляющие поверх места под фото — композицию объясняют только на кадре.",
        "sceneTemplate.item.recipeFourStep.title" => "Рецепт в четыре шага",
        "sceneTemplate.item.recipeFourStep.summary" => "Карточка 4:5, 2×2: все четыре шага на одной карточке. Сделайте скриншот и готовьте — у плиты листать неудобно.",
        "sceneTemplate.item.skincareRoutineCards.title" => "Карточки ухода за кожей",
        "sceneTemplate.item.skincareRoutineCards.summary" => "4:5, шесть кадров: в каждом шаге три числа — количество, время выдержки и утро или вечер. Ошибаются в дозе, а не в порядке.",
        "sceneTemplate.item.softwareStepTutorial.title" => "Пошаговая инструкция к программе",
        "sceneTemplate.item.softwareStepTutorial.summary" => "Карточка 4:5, единственная тёмная в серии: места под скриншоты и пронумерованные указания.",
        "sceneTemplate.item.storageMakeoverSteps.title" => "Шаги наведения порядка",
        "sceneTemplate.item.storageMakeoverSteps.summary" => "3:4, шесть кадров: кроме действия и картинки каждый шаг задаёт критерий готовности и бюджет времени.",
        "sceneTemplate.item.weeklyReportLesson.title" => "Урок по еженедельному отчёту",
        "sceneTemplate.item.weeklyReportLesson.summary" => "Высокая картинка: после разбора структуры из четырёх частей выдаёт каркас с подчёркнутыми пропусками.",
        "sceneTemplate.item.workoutBreakdownGuide.title" => "Разбор упражнений",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => "Высокая картинка: у каждого движения рядом с картинкой и подсказками стоит полоса подходов, повторов и отдыха.",
        "sceneTemplate.item.bookreviewSilkCarousel.title" => "Карусель разбора книги или фильма",
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4, пять полос: крючок, цитата с пометками, три наблюдения, одна фраза на вынос, финал. Работа разбирается на детали, а не пересказывается.",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "Карусель городского гида",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => "3:4, семь полос: места и маршруты чередуются — места для мечтающих, дневной маршрут и таблица «где есть и спать» для планирующих.",
        "sceneTemplate.item.datareportGridCarousel.title" => "Карусель отчёта по данным",
        "sceneTemplate.item.datareportGridCarousel.summary" => "3:4, шесть полос: после каждой полосы с данными идёт полоса без них, чтобы на третьем графике не пролистнули.",
        "sceneTemplate.item.opinionLongformCarousel.title" => "Карусель развёрнутого мнения",
        "sceneTemplate.item.opinionLongformCarousel.summary" => "3:4, шесть полос: строгий мастер-макет от начала до конца, номер и заголовок всегда на одном месте.",
        "sceneTemplate.item.qaChalkboardCarousel.title" => "Карусель вопросов и ответов",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => "3:4, шесть полос: один вопрос на полосу, в углу — нарисованный от руки номер-вопросительный знак.",
        "sceneTemplate.item.storyNightCarousel.title" => "Карусель личной истории",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4, семь полос: разбор пережитого по оси времени — лента на пятой полосе несущая, первые четыре её отметки.",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => "Карусель подборки инструментов",
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => "3:4, шесть полос: шесть инструментов по одному на полосу, последняя собирает их в оглавление с номерами.",
        "sceneTemplate.item.tutorialJournalCarousel.title" => "Карусель-инструкция",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => {
            "3:4, шесть полос: один шаг на полосу, палец и есть индикатор прогресса."
        }
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "Карусель итогов года",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => "3:4, восемь полос: полосы с цифрами холодные, полосы с размышлениями тёплые, вперемежку.",
        "fileMenu.newFromTemplate" => "Создать из шаблона",
        "fileMenu.exportSlideshowHtml" => "Экспортировать слайд-шоу в HTML...",
        "fileMenu.exportPptx" => "Экспортировать в PowerPoint...",
        "dialog.slideshowHtmlTitle" => "Экспорт слайд-шоу",
        "dialog.slideshowHtmlSummary" => "Экспортировано слайдов ({{count}}) в:",
        "dialog.slideshowHtmlEmpty" => "В этой презентации нет видимых слайдов для экспорта.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "Импортируемое содержимое HTML недоступно.",
        "htmlImport.warn.content.empty_body" => "Импортируемое содержимое в теле HTML недоступно.",
        "htmlImport.warn.content.dom_depth_truncated" => {
            "HTML с вложенностью глубже {{max_depth}} уровней отброшен."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Достигнут предел узлов; остальное содержимое страницы пропущено."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Достигнут предел узлов; часть дерева HTML пропущена."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Достигнут предел узлов; строка строчного форматирования пропущена."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Достигнут предел узлов; сгенерированные псевдоэлементы пропущены."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "Правила CSS с вложенностью глубже {{max_depth}} at-правил проигнорированы."
        }
        "htmlImport.warn.css.unterminated_rule" => "Незавершённое правило CSS проигнорировано.",
        "htmlImport.warn.css.marker_rules_unsupported" => "Правила CSS ::marker не импортированы.",
        "htmlImport.warn.css.nesting_unsupported" => {
            "Вложенные правила стилей CSS проигнорированы."
        }
        "htmlImport.warn.css.invalid_layer_name" => {
            "Недопустимое имя @layer '{{name}}' проигнорировано."
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "Неподдерживаемая инструкция @{{name}} проигнорирована."
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "Правила @media без области просмотра проигнорированы."
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "Недопустимое имя блока @layer '{{name}}' проигнорировано."
        }
        "htmlImport.warn.css.unsupported_container_block" => "Блок @container проигнорирован.",
        "htmlImport.warn.css.unsupported_block" => {
            "Неподдерживаемый блок @{{name}} проигнорирован."
        }
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "Веб-шрифт @font-face '{{family}}' недоступен."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Процентные смещения абсолютно позиционированного элемента аппроксимированы."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Процентные смещения position:relative аппроксимированы."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "Свойство CSS aspect-ratio без определённой оси проигнорировано."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "Свойство CSS aspect-ratio внутри неопределённого содержащего блока проигнорировано."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => {
            "Свойство CSS position:sticky проигнорировано."
        }
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "Неподдерживаемые дорожки CSS grid аппроксимированы."
        }
        "htmlImport.warn.layout.float_ignored" => "Свойство CSS float проигнорировано.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "Свойство CSS mix-blend-mode на уровне узла аппроксимировано."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "Свойство CSS overflow: auto / scroll аппроксимировано."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => {
            "Отрицательные внешние отступы CSS проигнорированы."
        }
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "Внешние отступы CSS у визуального блока проигнорированы."
        }
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "Процентные размеры content-box аппроксимированы."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "Пустые ячейки CSS grid, оставленные явными начальными линиями, аппроксимированы."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "Элемент CSS grid, охват которого не поместился от начальной линии, аппроксимирован."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Достигнут предел узлов; обёртки строк CSS grid пропущены."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "Ширины дорожек CSS grid с auto-fit / auto-fill аппроксимированы."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "Размещение по grid-template-areas не импортировано."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "Размещение по grid-row не импортировано."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "Свойство CSS grid-column `{{value}}` аппроксимировано."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "Автоматические отступы CSS по блочной оси не импортированы."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Достигнут предел узлов; выравнивание автоотступами CSS пропущено."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "Смещение CSS в потоке у элемента без определённого размера отброшено."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Достигнут предел узлов; смещение CSS в потоке пропущено."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "Смещения CSS в потоке (вставки position:relative, сдвиг transform) аппроксимированы."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "Смещение CSS в потоке у блока, не допускающего обёртку смещения, отброшено."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "Свойство flex-wrap в колоночном flex-контейнере не импортировано."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => {
            "Значение flex-wrap:wrap-reverse аппроксимировано."
        }
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "Свойство flex-wrap в контейнере без определённой ширины проигнорировано."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "Свойство CSS align-content в переносящем flex-контейнере не импортировано."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "Свойство flex-wrap при неопределённых размерах дочерних элементов по главной оси проигнорировано."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Достигнут предел узлов; строки flex-wrap пропущены."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "Неподдерживаемый синтаксис CSS transform проигнорирован."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "Неподдерживаемые функции CSS transform (3D, matrix3d) проигнорированы."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "Процентный сдвиг CSS transform по неопределённой оси отброшен."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "Преобразование CSS transform с неконечной матрицей проигнорировано."
        }
        "htmlImport.warn.transform.skew_dropped" => "Наклон CSS transform отброшен.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "Преобразование CSS transform с нулевым или неконечным масштабом аппроксимировано."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "Зеркальное отражение CSS transform аппроксимировано."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "Смещение по Z в CSS transform-origin проигнорировано."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "Масштаб CSS transform, который не удалось свести в размер узла, отброшен."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "Масштаб CSS transform, сведённый в размер узла, аппроксимирован."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "Масштаб CSS transform у элемента с авторазмером проигнорирован."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "Направленный или разреженный CSS background-repeat аппроксимирован."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "Явно заданный размер плитки фона CSS проигнорирован."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "Свойство CSS background-size у элемента с авторазмером аппроксимировано."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "Свойство CSS background-size, которому нужен собственный размер изображения, аппроксимировано."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "Неподдерживаемое значение CSS background-position проигнорировано."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "Пустой URL фонового изображения CSS проигнорирован."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => {
            "Конические градиенты CSS проигнорированы."
        }
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "Неподдерживаемый слой CSS background-image проигнорирован."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "Неразрешённый цвет фона CSS проигнорирован."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "Свойство CSS background-position проигнорировано."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "Цвета границ CSS по сторонам аппроксимированы."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Разные стили границ CSS по сторонам аппроксимированы."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "Сложный стиль границы CSS аппроксимирован."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "Неподдерживаемый стиль границы CSS аппроксимирован."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Эллиптические радиусы скругления CSS аппроксимированы."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "Неподдерживаемый радиус скругления CSS проигнорирован."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "Неподдерживаемый слой CSS box-shadow проигнорирован."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "Метод интерполяции цвета градиента CSS проигнорирован."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "Неподдерживаемое направление CSS linear-gradient проигнорировано."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "Цветовые подсказки градиентов CSS проигнорированы."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "Неподдерживаемая цветовая точка градиента CSS проигнорирована."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "Градиент CSS менее чем с двумя пригодными точками проигнорирован."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "Повторяющийся градиент CSS аппроксимирован."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "Точки градиента CSS вне диапазона аппроксимированы."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "Неподдерживаемый радиус размытия CSS проигнорирован."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "Неподдерживаемая функция CSS filter drop-shadow() проигнорирована."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "Неподдерживаемая функция CSS filter проигнорирована."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "Неподдерживаемая функция CSS backdrop-filter проигнорирована."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "Неподдерживаемое значение CSS background-blend-mode проигнорировано."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "Свойство CSS mix-blend-mode у отдельных заливок аппроксимировано."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "Неподдерживаемое значение CSS mix-blend-mode проигнорировано."
        }
        "htmlImport.warn.visual.property_not_representable" => {
            "Свойство CSS {{property}} проигнорировано."
        }
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "Свойство CSS background-size у градиента проигнорировано."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "Неподдерживаемая позиция CSS radial-gradient проигнорирована."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "Эллиптический CSS radial-gradient аппроксимирован."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "Ключевое слово протяжённости CSS radial-gradient аппроксимировано."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "Неподдерживаемый размер CSS radial-gradient проигнорирован."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "Неподдерживаемый слой CSS text-shadow проигнорирован."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "Слои CSS text-shadow после первого проигнорированы."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "Свойство CSS text-shadow у строчного элемента проигнорировано."
        }
        "htmlImport.warn.list.style_image_ignored" => {
            "Свойство CSS list-style-image не импортировано."
        }
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "Выносной маркер `list-style-position: outside` аппроксимирован."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "Неподдерживаемое значение CSS list-style-type `{{value}}` аппроксимировано."
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "Свойство CSS object-fit:scale-down аппроксимировано."
        }
        "htmlImport.warn.media.object_fit_none_ignored" => {
            "Свойство CSS object-fit:none проигнорировано."
        }
        "htmlImport.warn.media.object_position_ignored" => {
            "Свойство CSS object-position проигнорировано."
        }
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "Неподдерживаемое значение CSS mix-blend-mode у изображения проигнорировано."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "Встроенный элемент <svg> импортирован как заполнитель."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "Неподдерживаемый тип <input> аппроксимирован."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "Элемент <{{tag}}> импортирован как заполнитель."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "Элемент <picture> только с недекодируемыми типами источников аппроксимирован."
        }
        "htmlImport.warn.table.rowspan_ignored" => "Атрибут HTML rowspan не импортирован.",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "Ширины столбцов таблицы, группы строк которой развёрнуты стилями CSS, аппроксимированы."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "Ширины столбцов CSS-таблицы без определённой ширины аппроксимированы."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "Недопустимый <base href> {{href}} проигнорирован."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "Элемент <base href> {{href}} вне источника проекта проигнорирован."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "Внешняя таблица стилей {{url}} недоступна."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "Изображение {{url}} вне источника проекта импортировано как заполнитель."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "Недоступное изображение {{url}} импортировано как заполнитель."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "Недопустимый CSS @import {{prelude}} проигнорирован."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "CSS @import {{reference}} недоступен."
        }
        "htmlImport.warn.resource.css_import_cycle" => {
            "Циклический CSS @import {{url}} проигнорирован."
        }
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "CSS @import {{url}} за пределом глубины {{max_depth}} проигнорирован."
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} недоступен.",
        "htmlImport.warn.project.multiple_html_entries" => {
            "Найдено точек входа HTML: {{count}}; выбрана {{entry}}, остальные аппроксимированы."
        }
        "htmlImport.warn.snapshot.truncated" => "Часть снимка браузера отброшена.",
        "htmlImport.warn.snapshot.node_limit" => {
            "Достигнут предел узлов; остальное содержимое снимка пропущено."
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "Изображений с ограничениями CORS, сохранённых как удалённые URL и недоступных: {{count}}."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "Узел снимка с отсутствующим или недопустимым прямоугольником отброшен."
        }
        "htmlImport.warn.snapshot.unknown_kind" => "Узел снимка неизвестного вида отброшен.",
        "htmlImport.warn.snapshot.rejected" => "Снимок браузера ({{reason}}) отброшен.",
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "Неподдерживаемое преобразование в снимке проигнорировано."
        }
        "htmlImport.warn.css.media_empty_query" => "Пустой запрос @media проигнорирован.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "Неподдерживаемый тип @media '{{name}}' проигнорирован."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "Неподдерживаемое условие @media '{{input}}' проигнорировано."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "Недопустимая ориентация @media '{{value}}' проигнорирована."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "Неподдерживаемая характеристика @media '{{name}}' проигнорирована."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "Неподдерживаемый диапазон @media '({{input}})' проигнорирован."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "Недопустимый диапазон @media '({{input}})' проигнорирован."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "Недопустимая длина @media '{{value}}' проигнорирована."
        }
        "htmlImport.diagnostics.title" => "Импорт HTML завершён",
        "htmlImport.diagnostics.summary" => "Ухудшенных элементов: {{count}}",
        "htmlImport.diagnostics.dismiss" => "Закрыть",
        "htmlImport.diagnostics.expand" => "Показать подробности",
        "htmlImport.diagnostics.collapse" => "Скрыть подробности",
        "htmlImport.diagnostics.more" => "Ещё {{count}}",
        "dialog.pptxTitle" => "Экспорт в PowerPoint",
        "dialog.pptxSummary" => "Экспортировано слайдов ({{count}}) в:",
        "dialog.pptxEmpty" => "В этой презентации нет видимых слайдов для экспорта.",
        "settings.agents.acpQuickAdd" => "Быстрое добавление",
        "settings.agents.acpPresetAdd" => "Добавить",
        "settings.agents.acpNotInstalled" => "Не установлено",
        "assetCenter.title" => "Центр ресурсов",
        "assetCenter.tab.templates" => "Шаблоны",
        "assetCenter.tab.styles" => "Стили",
        "assetCenter.style.empty" => "Подходящих стилей нет",
        "assetCenter.style.pinned" => "Закреплён",
        "assetCenter.style.searchPlaceholder" => "Поиск стилей или тегов",
        "assetCenter.style.generateHint" => "Новый документ по вашей теме в закреплённом стиле.",
        "ai.pinnedStyle" => "Стиль: {{name}}",
        "assetCenter.style.import" => "Импорт стиля",
        "assetCenter.style.mine" => "Мои стили",
        "assetCenter.style.builtIn" => "Встроенные стили",
        "assetCenter.style.importTitle" => "Импорт DESIGN.md",
        "assetCenter.style.importHint" => "Вставьте весь DESIGN.md и подтвердите импорт.",
        "assetCenter.style.importSource" => "Стиль можно скопировать из библиотеки DESIGN.md, например styles.refero.design.",
        "assetCenter.style.importConfirm" => "Импортировать",
        "assetCenter.style.importCancel" => "Отмена",
        "assetCenter.style.importPickFile" => "Выбрать файл…",
        "assetCenter.style.importHintFile" => "Выберите файл DESIGN.md или вставьте весь документ ниже.",
        "assetCenter.style.importPlaceholder" => "Вставьте DESIGN.md сюда",
        "assetCenter.style.importEmpty" => "Файл пуст или слишком короткий для руководства по стилю.",
        "assetCenter.style.importNotText" => "Файл не читается как текст Markdown.",
        "assetCenter.style.importTooLarge" => "Файл больше 512 КБ.",
        "slidesPanel.tabSlides" => "Слайды",
        "slidesPanel.tabCards" => "Карточки",
        "slidesPanel.present" => "Показать",
        "slidesPanel.exportPdf" => "Экспорт в PDF",
        "slidesPanel.exportAllSlides" => "Экспортировать все слайды",
        "slidesPanel.exportSelectedSlides" => "Экспортировать выбранные слайды ({{count}})",
        "settings.tab.ai" => "ИИ",
        "settings.agents.heroTitle" => "Подключите своего AI-провайдера",
        "settings.agents.heroSubtitle" => "OpenPencil управляет локальными CLI-агентами и API-провайдерами — подключите любого, чтобы генерировать дизайны.",
        "settings.agents.statusConnected" => "Подключено",
        "settings.agents.statusNotConnected" => "Не подключено",
        "settings.agents.statusChecking" => "Проверка…",
        "settings.mcp.heroTitle" => "Подключите OpenPencil извне по MCP",
        "settings.mcp.heroSubtitle" => "Направьте любой поддерживающий MCP CLI или редактор на это рабочее пространство и управляйте холстом теми же инструментами, что и встроенный агент.",
        "settings.mcp.terminalFootnote" => "* При запуске MCP автоматически настраивается для выбранных CLI-инструментов.",
        "settings.mcp.customConfigTitle" => "Своя конфигурация MCP-сервера",
        "settings.mcp.customConfigDesc" => "Вставьте это в любой клиент, который читает стандартный блок MCP server.",
        "settings.mcp.copyConfig" => "Копировать конфиг MCP",
        "settings.system.heroTitle" => "Системные настройки",
        "settings.system.heroSubtitle" => "Оформление, обновления и поведение холста для этой установки.",
        "settings.system.appearance" => "Оформление",
        "settings.system.appearanceLight" => "Светлое",
        "settings.system.appearanceDark" => "Тёмное",
        "settings.system.pencilCursor" => "Курсор карандаша",
        "settings.images.heroTitle" => "Изображения для дизайнов",
        "settings.images.heroSubtitle" => "Ищите фото в Openverse или подключите провайдера, чтобы генерировать их по запросу.",
        "settings.fonts.heroTitle" => "Шрифты в этом документе",
        "settings.fonts.heroSubtitle" => "Подберите шрифты, которых требует документ, но нет на этом компьютере, и управляйте импортированными.",
        "settings.account.heroTitle" => "Ваш аккаунт",
        "settings.account.heroSubtitle" => "Войдите, чтобы синхронизировать рабочее пространство и лицензию между устройствами.",
        "tooltip.topbar.file" => "Файл",
        "tooltip.topbar.import" => "Импорт",
        "tooltip.topbar.language" => "Язык",
        "tooltip.topbar.collaboration" => "Совместная работа",
        "tooltip.topbar.preview" => "Просмотр",
        "tooltip.topbar.exitPreview" => "Выйти из просмотра",
        "tooltip.topbar.account" => "Аккаунт",
        "settings.agents.providerRollMore" => "и ещё {{count}}",
        "ai.thinking.adaptive" => "Размышление: авто",
        "ai.thinking.disabled" => "Размышление: выкл.",
        "ai.thinking.enabled" => "Размышление: вкл.",
        "ai.designProgress.detail.repairsApplied" => "Применено автоисправлений: {{count}}",
        "ai.designProgress.detail.repairsMore" => "… и ещё {{count}} (см. журнал)",
        "ai.styleCard.builtin" => "Встроенный стиль",
        "ai.styleCard.imported" => "Импортированный DESIGN.md",
        "ai.styleCard.documentDesignMd" => "design.md документа",
        _ => return super::ru_collab::lookup(key),
    })
}
