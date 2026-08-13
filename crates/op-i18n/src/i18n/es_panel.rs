//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `es_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Buscar imágenes…",
        "imagePanel.searching" => "Buscando…",
        "imagePanel.noResults" => "Sin resultados",
        "imagePanel.searchPrompt" => "Busca imágenes",
        "imagePanel.sourceNotice" => {
            "Imágenes de {{source}}. Licencia libre — verifica la licencia antes de usar."
        }
        "imagePanel.genNotConfigured" => "La generación de imágenes no está configurada",
        "imagePanel.openSettings" => "Abrir ajustes",
        "imagePanel.promptPlaceholder" => "Describe la imagen…",
        "providerProbe.connectedViaCli" => "Conectado a través de la CLI de {{name}}",
        "providerProbe.cliExitedWithError" => "La CLI de {{name}} finalizó con un error",
        "providerProbe.cliNoVersionOutput" => "La CLI de {{name}} no devolvió información de versión",
        "providerProbe.modelQueryFailed" => "La consulta de modelos de {{name}} falló o superó el tiempo de espera",
        "providerProbe.modelQueryFailedRunLogin" => "La consulta de modelos de {{name}} falló. Ejecuta {{command}} una vez para autenticarte.",
        "providerProbe.modelQueryNeedsAuth" => "La consulta de modelos de {{name}} requiere autenticación. Ejecuta {{command}} una vez para iniciar sesión.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} devolvió un catálogo de modelos no reconocido",
        "promptCenter.title" => "Centro de prompts",
        "promptCenter.searchPlaceholder" => "Buscar prompts…",
        "promptCenter.category.all" => "Todo",
        "promptCenter.category.starter" => "Inicio rápido",
        "promptCenter.category.mobileApp" => "App móvil",
        "promptCenter.category.webPage" => "Página web",
        "promptCenter.category.dashboard" => "Panel",
        "promptCenter.category.component" => "Componente",
        "promptCenter.category.modify" => "Modificar",
        "promptCenter.category.custom" => "Mis prompts",
        "promptCenter.empty" => "No hay prompts que coincidan",
        "promptCenter.saveCurrent" => "Guardar el texto actual como prompt",
        "promptCenter.saveTitlePlaceholder" => "Título del prompt",
        "promptCenter.save" => "Guardar",
        "promptCenter.cancel" => "Cancelar",
        "promptCenter.delete" => "Eliminar",
        "promptCenter.screens" => "{{count}} pantallas",
        "promptCenter.freeform" => "Formato libre",
        "promptCenter.item.wander.title" => "Wander · Itinerarios de viaje",
        "promptCenter.item.forage.title" => "Forage · Recetas de temporada",
        "promptCenter.item.still.title" => "Still · Meditación y descanso",
        "promptCenter.item.hearth.title" => "Hearth · Hogar inteligente",
        "promptCenter.item.meteo.title" => "Meteo · Tiempo inmersivo",
        "promptCenter.item.marginalia.title" => "Marginalia · Lectura y anotaciones",
        "promptCenter.item.lingua.title" => "Lingua · Aprendizaje de idiomas",
        "promptCenter.item.daybreak.title" => "Daybreak · Pedidos de café",
        "promptCenter.item.verdant.title" => "Verdant · Cuidado de plantas",
        "promptCenter.item.companion.title" => "Companion · Vida con mascotas",
        "promptCenter.item.relic.title" => "Relic · Mercado selecto de segunda mano",
        "promptCenter.item.nocturne.title" => "Nocturne · Guía de observación estelar",
        "promptCenter.item.marquee.title" => "Marquee · Lista de películas",
        "promptCenter.item.ritual.title" => "Ritual · Creación de hábitos",
        "promptCenter.item.ember.title" => "Ember · Diario de estados de ánimo",
        "promptCenter.item.volt.title" => "Volt · Compañero para vehículos eléctricos",
        "promptCenter.item.aloft.title" => "Aloft · Seguimiento de vuelos",
        "promptCenter.item.gallery.title" => "Gallery · Exposiciones y cultura",
        "promptCenter.item.nightcap.title" => "Nightcap · Coctelería en casa",
        "promptCenter.item.bloom.title" => "Bloom · Seguimiento del crecimiento familiar",
        "promptCenter.item.extremeWeather.title" => "Extremo · App del tiempo",
        "promptCenter.item.extremeNowPlaying.title" => "Extremo · En reproducción",
        "promptCenter.item.extremeDailyApp.title" => "Extremo · Para abrir cada día",
        "promptCenter.item.extremeCalendar.title" => "Extremo · Reinventar el calendario",
        "promptCenter.item.extremeCalm.title" => "Extremo · Una pantalla de calma",
        "promptCenter.item.webOrbit.title" => "Orbit · Página de inicio del espacio de trabajo de IA",
        "promptCenter.item.webAtelier.title" => "Atelier · Comercio de muebles",
        "promptCenter.item.webKilnform.title" => "Kilnform · Sitio de infraestructura de diseño",
        "promptCenter.item.webReefwright.title" => "Reefwright · Sitio de conocimiento de soporte con IA",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Panel de analítica de crecimiento",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Operaciones logísticas",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Tabla de datos empresarial",
        "promptCenter.item.componentFormLab.title" => {
            "Form Lab · Sistema de componentes de formulario"
        }
        "promptCenter.item.modifyPolishCurrent.title" => "Pulir la pantalla actual",
        "promptCenter.item.modifyCompleteStates.title" => "Completar los estados de los componentes",
        "collab.ownerConfirm.title" => "Confirma a quién te vas a unir",
        "collab.ownerConfirm.hint" => "Todavía no se ha cargado nada de esta sesión.",
        "collab.ownerConfirm.account" => "Cuenta verificada",
        "collab.ownerConfirm.device" => "Dispositivo verificado",
        "collab.ownerConfirm.claimedName" => "Nombre elegido por esta cuenta (sin verificar)",
        "collab.action.confirmOwner" => "Unirse a esta sesión",
        "collab.action.rejectOwner" => "No unirse",
        "collab.error.ownerNotConfirmed" => "No confirmaste al anfitrión, así que no se cargó nada.",
        "sceneTemplate.title" => "Plantillas de escenas",
        "sceneTemplate.searchPlaceholder" => "Buscar escenas o plantillas…",
        "sceneTemplate.empty" => "No hay plantillas que coincidan",
        "sceneTemplate.frames" => "Páginas: {{count}}",
        "sceneTemplate.generate.placeholder" => "Describe un tema y la IA genera toda la presentación",
        "sceneTemplate.generate.button" => "Generar",
        "sceneTemplate.generate.hint" => "Un documento nuevo, creado a partir de tu tema como presentación completa.",
        "sceneTemplate.generate.promptTemplate" => "Crea una presentación (PPT) sobre el siguiente tema: {{topic}}",
        "sceneTemplate.card.addToCanvas" => "Añadir al lienzo",
        "sceneTemplate.card.generateFrom" => "Generar con este",
        "sceneTemplate.generate.basis" => "Basado en: ",
        "sceneTemplate.filter.all" => "Todo",
        "sceneTemplate.scene.tutorial" => "Tutoriales",
        "sceneTemplate.scene.comparison" => "Comparación",
        "sceneTemplate.scene.carousel" => "Carrusel",
        "sceneTemplate.scene.slides" => "Diapositivas",
        "sceneTemplate.scene.card" => "Tarjetas",
        "sceneTemplate.scene.web" => "Páginas web",
        "sceneTemplate.generate.webPromptTemplate" => "Diseña una página de aterrizaje web de varias secciones sobre el siguiente tema: {{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "Página SaaS · Naranja",
        "sceneTemplate.item.saasLandingOrange.summary" => "Una página de marketing clara construida sobre paneles casi negros y un solo naranja: navegación, hero con captura del producto, tres tarjetas de capacidades, un recorrido por el flujo, testimonios y un pie de suscripción. Cambia los textos y ya es un sitio.",
        "sceneTemplate.item.productLandingLight.title" => "Página de producto · Clara",
        "sceneTemplate.item.productLandingLight.summary" => "Una página de producto blanco papel, de aire editorial: demo interactiva en el hero, columnas de capacidades, panel de analítica, comparativa antes/después y tres planes de precio. Para sitios SaaS y lanzamientos.",
        "sceneTemplate.item.screenshotTutorial.title" => "Tutorial con capturas · 3 pasos",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Portada, tres pasos y una llamada a la acción final. Sustituye las capturas y el texto para publicar."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Carrusel de conocimiento e ideas",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Portada, tres ideas clave y una página de resumen, ideal para convertir un punto de vista en tarjetas para deslizar."
        }
        "sceneTemplate.item.beforeAfter.title" => "Comparativa antes/después",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Comparativa antes/después en paralelo, con notas sobre los cambios; ideal para retrospectivas y portfolios."
        }
        "sceneTemplate.item.slideDeck.title" => "Presentación · 6 diapositivas",
        "sceneTemplate.item.slideDeck.summary" => {
            "Portada, agenda, puntos clave, datos, gráfico y cierre, en formato 16:9. Sustituye el texto y estará lista para presentar."
        }
        "sceneTemplate.item.knowledgeCardVertical.title" => "Tarjeta de conocimiento · Vertical",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "Una sola tarjeta 3:4 con titular, cuatro ideas clave y una firma. Cambia el texto y publícala.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "Tarjeta de conocimiento · Cuadrada",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "Una tarjeta 1:1 con la misma composición, compacta para una cabecera o una publicación social.",
        "sceneTemplate.item.pitchDeckDark.title" => "Pitch deck · Oscuro",
        "sceneTemplate.item.pitchDeckDark.summary" => "Portada, problema, solución, cifras, hoja de ruta y página de contacto. Tipografía grande sobre fondo oscuro, pensado para rondas de inversión.",
        "sceneTemplate.item.lectureDeckLight.title" => "Material de clase · Claro",
        "sceneTemplate.item.lectureDeckLight.summary" => "Portada del curso, objetivos, explicación de un concepto, ejercicio resuelto, tabla comparativa y cierre. Fondo blanco papel, cómodo durante toda la sesión.",
        "sceneTemplate.item.minimalKeynote.title" => "Keynote minimalista",
        "sceneTemplate.item.minimalKeynote.summary" => "Espacio en blanco, tipografía enorme y una frase centrada por página: nueve páginas sin una sola tarjeta y un índice de puras líneas y cifras. Para lanzamientos y conferencias.",
        "sceneTemplate.item.gradientTech.title" => "Tech degradado",
        "sceneTemplate.item.gradientTech.summary" => "Fondo degradado oscuro con tarjetas de vidrio esmerilado: arquitectura, rendimiento y muro de clientes. Para lanzamientos de producto técnico.",
        "sceneTemplate.scene.infographic" => "Infografías",
        "sceneTemplate.item.punchQuoteCard.title" => "Tarjeta de cita · Cartel",
        "sceneTemplate.item.punchQuoteCard.summary" => "Una tarjeta 3:4 sobre fondo casi negro: dos líneas enormes sobre una franja amarilla. Una sola frase, para opiniones y citas.",
        "sceneTemplate.item.journalChecklistCard.title" => {
            "Tarjeta de tareas · Base de conocimiento"
        }
        "sceneTemplate.item.journalChecklistCard.summary" => "Una tarjeta blanca sobre fondo gris claro: cinco tareas para marcar, una etiqueta y una cita. Para planes semanales.",
        "sceneTemplate.item.dataReportInfographic.title" => "Infografía de resultados",
        "sceneTemplate.item.dataReportInfographic.summary" => "Una imagen vertical para desplazar: cabecera oscura, tres cifras grandes, una comparativa de barras, un reparto y tres conclusiones. Cambia los números y publícala.",
        "sceneTemplate.item.stepsFlowInfographic.title" => "Infografía paso a paso",
        "sceneTemplate.item.stepsFlowInfographic.summary" => "Una imagen vertical para desplazar: cinco tarjetas numeradas encadenadas en un flujo, cada una con su duración, más dos consejos. Para tutoriales y guías.",
        "sceneTemplate.item.eventPosterDeck.title" => "Deck de evento · Cartel",
        "sceneTemplate.item.eventPosterDeck.summary" => "Portada, atractivos, programa, cómo llegar, entradas y cierre. Fondo blanco de galería con bloques rojos y azules, sin esquinas redondeadas ni degradados — para mercados, eventos y aperturas.",
        "sceneTemplate.item.pitfallListInfographic.title" => "Infografía de errores a evitar",
        "sceneTemplate.item.pitfallListInfographic.summary" => "Una imagen vertical para desplazar: seis errores ordenados por frecuencia, cada uno con qué falla y qué hacer en su lugar, más una comprobación de cuatro puntos antes de publicar. Solo blanco, negro y gris.",
        "sceneTemplate.item.spineCultureCard.title" => {
            "Tarjeta de título vertical · Pigmento mineral"
        }
        "sceneTemplate.item.spineCultureCard.summary" => "Una tarjeta 3:4 sobre fondo de arcilla ocre: título chino en vertical, yeso descascarillado y granos de pigmento. Para cultura, textos largos y portadas de autor.",
        "sceneTemplate.item.metricSingleCard.title" => "Tarjeta de dato único · Retícula Hanzi",
        "sceneTemplate.item.metricSingleCard.summary" => "Una tarjeta 1:1: un número enorme sobre blanco puro, una retícula suiza estricta y un solo cuadrado rojo de señal. Para conclusiones y resultados.",
        "sceneTemplate.item.quoteFrameCard.title" => "Tarjeta de cita · Seda azul-verde",
        "sceneTemplate.item.quoteFrameCard.summary" => "Una tarjeta 4:5 sobre seda amarilleada: una frase enmarcada y, al pie, una montaña de azurita y malaquita. Para extractos, entrevistas y citas.",
        "sceneTemplate.item.dailySignCard.title" => "Tarjeta diaria · Ventana de jardín",
        "sceneTemplate.item.dailySignCard.summary" => "Una tarjeta 3:4 sobre muro encalado con una celosía hexagonal: dentro, la fecha y una línea. El vacío es el adorno. Para publicaciones diarias y lemas.",
        "sceneTemplate.item.priceTierCard.title" => "Tarjeta de precios · Neón de soportales",
        "sceneTemplate.item.priceTierCard.summary" => "Una tarjeta 1:1 sobre noche azul tinta: tres niveles de precios, contornos de tubos de neón y su halo. Para tiendas, eventos y paquetes.",
        "sceneTemplate.item.noticeBoardCard.title" => "Tarjeta de aviso · Tipos de plomo",
        "sceneTemplate.item.noticeBoardCard.summary" => "Una tarjeta 4:5 sobre papel de periódico: filetes de cabecera con registro desviado, cláusulas numeradas y un sello de serie. Para avisos y normas.",
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "Infografía de línea temporal",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "Una imagen vertical para desplazar: un eje que recorre toda la altura, marcas de año junto a las tarjetas de hitos y un cierre con lo que viene. Para balances, historia de marca y trayectoria de proyecto.",
        "sceneTemplate.item.conceptContrastInfographic.title" => {
            "Infografía de contraste de conceptos"
        }
        "sceneTemplate.item.conceptContrastInfographic.summary" => "Una imagen vertical para desplazar: primero la conclusión, luego una tarjeta de definición por concepto, un desglose a dos columnas por criterio y, al final, cómo elegir.",
        "sceneTemplate.item.rankingBoardInfographic.title" => "Infografía de ranking Top N",
        "sceneTemplate.item.rankingBoardInfographic.summary" => "Una imagen vertical para desplazar: un tablero dorado sobre tinta — insignias grandes para los tres primeros y de contorno del cuarto al octavo, con cuándo usarlo y con qué frecuencia.",
        "sceneTemplate.item.faqThreadInfographic.title" => "Infografía de preguntas frecuentes",
        "sceneTemplate.item.faqThreadInfographic.summary" => "Una imagen vertical para desplazar: seis pares de pregunta y respuesta, P sólida y R de contorno. Sin numeración ni orden: cada par se sostiene solo.",
        "sceneTemplate.item.dataStoryInfographic.title" => "Infografía de relato de datos",
        "sceneTemplate.item.dataStoryInfographic.summary" => "Una imagen vertical para desplazar: cuatro cifras encadenadas en una línea causal, cada tramo como una rejilla de diez bloques, y una conclusión accionable al cierre.",
        "sceneTemplate.item.challengeTrackerInfographic.title" => "Infografía de reto de 30 días",
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "Una imagen vertical para desplazar: una rejilla de treinta casillas, seis por cinco, con hitos solo en los días 7, 15 y 30. Guárdala y tacha una al día.",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "Infografía de mapa de ecosistema",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => "Una imagen vertical para desplazar: cuatro posiciones de una misma cadena en dos por dos, tres actores en cada una y los huecos señalados. Tarjetas blancas sobre pizarra.",
        "sceneTemplate.item.doDontComparison.title" => "Dos columnas: bien y mal",
        "sceneTemplate.item.doDontComparison.summary" => "Una tarjeta 3:4: dos maneras de hacer lo mismo, distinguidas por material e icono en lugar de rojo contra verde, legible también para daltónicos.",
        "sceneTemplate.item.mythTruthComparison.title" => "Mitos y realidad",
        "sceneTemplate.item.mythTruthComparison.summary" => "Una imagen alta: cinco pares «se suele decir / en realidad», el mito estrecho y claro a la izquierda, la realidad ancha y oscura a la derecha.",
        "sceneTemplate.item.pricingTiersComparison.title" => "Comparativa de planes",
        "sceneTemplate.item.pricingTiersComparison.summary" => "Una tarjeta 3:4: Gratis, Pro y Equipo en paralelo, el precio como ancla, cada columna contiene la anterior. Para páginas de precios.",
        "sceneTemplate.item.scenarioGuideComparison.title" => "Guía de elección por situación",
        "sceneTemplate.item.scenarioGuideComparison.summary" => "Una imagen alta: sin especificaciones, siete situaciones, cada una con su veredicto. El lector solo busca su fila.",
        "sceneTemplate.item.specTableComparison.title" => "Tabla comparativa de especificaciones",
        "sceneTemplate.item.specTableComparison.summary" => "Una imagen alta: dos candidatos en una tabla real, fila a fila, con la celda ganadora realzada sobre fondo oscuro.",
        "sceneTemplate.item.threeWayComparison.title" => "Comparativa de tres opciones",
        "sceneTemplate.item.threeWayComparison.summary" => "Una imagen alta: tres opciones en paralelo con la recomendada en el centro; cada columna abre con una situación, no con un nombre.",
        "sceneTemplate.item.timeShiftComparison.title" => "Hace un año y ahora",
        "sceneTemplate.item.timeShiftComparison.summary" => "Una tarjeta 3:4: una espina central de etiquetas, hace un año a la izquierda y ahora a la derecha, ambos valores en la misma fila.",
        "sceneTemplate.item.tradeoffScaleComparison.title" => "Balanza de pros y contras",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => "Una tarjeta 1:1: un brazo y dos platillos — lo que vale a la izquierda, lo que cuesta a la derecha, una casilla vacía ante cada línea.",
        "sceneTemplate.item.versionDiffComparison.title" => "Cambios entre versiones",
        "sceneTemplate.item.versionDiffComparison.summary" => "Una tarjeta 1:1: sin columnas — cada fila completa su propio «antes → después»; basta con desplazarse.",
        "sceneTemplate.item.appOnboardingTriptych.title" => "Tríptico de onboarding de app",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "Una tarjeta 3:4: tres teléfonos en fila con huecos de imagen vacíos. Coloca tus tres pantallas, añade el texto y ya sirve para revisión o publicación.",
        "sceneTemplate.item.diyBlueprintGuide.title" => "Guía DIY ilustrada",
        "sceneTemplate.item.diyBlueprintGuide.summary" => "Una imagen alta donde la tabla de materiales ocupa tanto como los pasos: el bricolaje falla en la preparación, no en las manos.",
        "sceneTemplate.item.photoCompositionTutorial.title" => "Composición fotográfica con móvil",
        "sceneTemplate.item.photoCompositionTutorial.summary" => "3:4, cinco cuadros: cada uno un visor oscuro con guías fluorescentes sobre el hueco de la foto, porque la composición solo se explica sobre el encuadre.",
        "sceneTemplate.item.recipeFourStep.title" => "Receta en cuatro pasos",
        "sceneTemplate.item.recipeFourStep.summary" => "Una tarjeta 4:5 en 2×2: los cuatro pasos en una sola tarjeta. Captura y cocina — frente a los fogones pasar páginas estorba.",
        "sceneTemplate.item.skincareRoutineCards.title" => "Tarjetas de rutina de cuidado",
        "sceneTemplate.item.skincareRoutineCards.summary" => "4:5, seis cuadros: cada paso fija tres números — cantidad, espera y si es de día o de noche. Se falla en la dosis, no en el orden.",
        "sceneTemplate.item.softwareStepTutorial.title" => "Tutorial de software paso a paso",
        "sceneTemplate.item.softwareStepTutorial.summary" => "Una tarjeta 4:5, la única oscura de la serie: huecos de captura con instrucciones numeradas, para herramientas y funciones.",
        "sceneTemplate.item.storageMakeoverSteps.title" => "Pasos de reforma del orden",
        "sceneTemplate.item.storageMakeoverSteps.summary" => "3:4, seis cuadros: además del gesto y la imagen, cada paso fija un criterio de fin y un presupuesto de tiempo.",
        "sceneTemplate.item.weeklyReportLesson.title" => "Lección de informe semanal",
        "sceneTemplate.item.weeklyReportLesson.summary" => "Una imagen alta: tras la estructura en cuatro partes entrega un esqueleto con huecos subrayados — captura y rellena.",
        "sceneTemplate.item.workoutBreakdownGuide.title" => "Guía de desglose de ejercicios",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => "Una imagen alta: cada movimiento lleva una barra fija de series / repeticiones / descanso junto a la imagen y las claves.",
        "sceneTemplate.item.bookreviewSilkCarousel.title" => {
            "Carrusel de análisis de libro / película"
        }
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4, cinco tableros: gancho, extracto anotado, tres ideas, una frase citable, cierre. Desmonta la obra en piezas que llevarse, no resume la trama.",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "Carrusel de guía de ciudad",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => "3:4, siete tableros: lugares y rutas alternan — los lugares para quien sueña, la ruta del día y la tabla de comer y dormir para quien planifica.",
        "sceneTemplate.item.datareportGridCarousel.title" => "Carrusel de informe de datos",
        "sceneTemplate.item.datareportGridCarousel.summary" => "3:4, seis tableros: cada tablero de datos va seguido de uno sin datos, para que nadie se salte el tercer gráfico.",
        "sceneTemplate.item.opinionLongformCarousel.title" => "Carrusel de opinión larga",
        "sceneTemplate.item.opinionLongformCarousel.summary" => "3:4, seis tableros: una maqueta estricta de principio a fin, número y título siempre en el mismo sitio.",
        "sceneTemplate.item.qaChalkboardCarousel.title" => "Carrusel de preguntas y respuestas",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => "3:4, seis tableros: una pregunta por tablero, con un número-interrogación dibujado a mano en la esquina.",
        "sceneTemplate.item.storyNightCarousel.title" => "Carrusel de relato",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4, siete tableros: un repaso personal construido sobre el tiempo — la línea temporal del quinto tablero es el muro de carga.",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => {
            "Carrusel de recopilación de herramientas"
        }
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => "3:4, seis tableros: seis herramientas una por tablero, y el último las lista con sus números de página.",
        "sceneTemplate.item.tutorialJournalCarousel.title" => "Carrusel de tutorial",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => {
            "3:4, seis tableros: un paso por tablero, el dedo es la barra de progreso."
        }
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "Carrusel de balance anual",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => "3:4, ocho tableros: tableros de cifras fríos y tableros de reflexión cálidos, alternándose.",
        "fileMenu.newFromTemplate" => "Nuevo a partir de una plantilla",
        "fileMenu.exportSlideshowHtml" => "Exportar presentación HTML...",
        "fileMenu.exportPptx" => "Exportar a PowerPoint...",
        "dialog.slideshowHtmlTitle" => "Exportar presentación",
        "dialog.slideshowHtmlSummary" => "Se exportaron {{count}} diapositivas a:",
        "dialog.slideshowHtmlEmpty" => "Esta presentación no tiene diapositivas visibles para exportar.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "El contenido HTML importable no está disponible.",
        "htmlImport.warn.content.empty_body" => {
            "El contenido importable del cuerpo HTML no está disponible."
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "Se descartó el HTML anidado a más de {{max_depth}} niveles."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Se alcanzó el límite de nodos; se omitió el resto del contenido de la página."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Se alcanzó el límite de nodos; se omitió parte del árbol HTML."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Se alcanzó el límite de nodos; se omitió una fila de formato en línea."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Se alcanzó el límite de nodos; se omitieron los pseudoelementos generados."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "Se ignoraron las reglas CSS anidadas en más de {{max_depth}} reglas @."
        }
        "htmlImport.warn.css.unterminated_rule" => "Se ignoró una regla CSS sin terminar.",
        "htmlImport.warn.css.marker_rules_unsupported" => {
            "No se importaron las reglas CSS ::marker."
        }
        "htmlImport.warn.css.nesting_unsupported" => {
            "Se ignoraron las reglas de estilo CSS anidadas."
        }
        "htmlImport.warn.css.invalid_layer_name" => {
            "Se ignoró el nombre de @layer no válido '{{name}}'."
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "Se ignoró la instrucción @{{name}} no admitida."
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "Se ignoraron las reglas @media sin ventana gráfica."
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "Se ignoró el nombre de bloque @layer no válido '{{name}}'."
        }
        "htmlImport.warn.css.unsupported_container_block" => "Se ignoró el bloque @container.",
        "htmlImport.warn.css.unsupported_block" => "Se ignoró el bloque @{{name}} no admitido.",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "La fuente web @font-face '{{family}}' no está disponible."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Se aproximaron los desplazamientos en porcentaje de un elemento con posición absoluta."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Se aproximaron los desplazamientos en porcentaje de position:relative."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "Se ignoró el aspect-ratio CSS sin un eje definido."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "Se ignoró el aspect-ratio CSS dentro de un bloque contenedor indefinido."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "Se ignoró position:sticky de CSS.",
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "Se aproximaron las pistas de cuadrícula CSS no admitidas."
        }
        "htmlImport.warn.layout.float_ignored" => "Se ignoró float de CSS.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "Se aproximó mix-blend-mode de CSS a nivel de nodo."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "Se aproximó overflow: auto / scroll de CSS."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => {
            "Se ignoraron los márgenes CSS negativos."
        }
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "Se ignoraron los márgenes CSS de una caja visual."
        }
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "Se aproximó el dimensionamiento en porcentaje de content-box."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "Se aproximaron las celdas vacías de cuadrícula CSS dejadas por líneas de inicio explícitas."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "Se aproximó un elemento de cuadrícula CSS cuya extensión no cabía en su línea de inicio."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Se alcanzó el límite de nodos; se omitieron los envoltorios de filas de cuadrícula CSS."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "Se aproximaron los anchos de pista de cuadrícula CSS con auto-fit / auto-fill."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "No se importó la colocación con grid-template-areas de CSS."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "No se importó la colocación con grid-row de CSS."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "Se aproximó grid-column `{{value}}` de CSS."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "No se importaron los márgenes automáticos CSS en el eje de bloque."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Se alcanzó el límite de nodos; se omitió la alineación por márgenes automáticos CSS."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "Se descartó un desplazamiento CSS en flujo sobre un elemento sin tamaño definido."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Se alcanzó el límite de nodos; se omitió un desplazamiento CSS en flujo."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "Se aproximaron los desplazamientos CSS en flujo (separaciones de position:relative, traslación de transform)."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "Se descartó un desplazamiento CSS en flujo en una caja que no admite un envoltorio de desplazamiento."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "No se importó flex-wrap en un contenedor flex en columna."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => "Se aproximó flex-wrap:wrap-reverse.",
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "Se ignoró flex-wrap en un contenedor sin ancho definido."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "No se importó align-content de CSS en un contenedor flex con ajuste de línea."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "Se ignoró flex-wrap con tamaños indeterminados de los hijos en el eje principal."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Se alcanzó el límite de nodos; se omitieron las filas de flex-wrap."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "Se ignoró una sintaxis de transform CSS no admitida."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "Se ignoraron las funciones de transform CSS no admitidas (3D, matrix3d)."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "Se descartó una traslación en porcentaje de transform CSS sobre un eje indefinido."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "Se ignoró un transform CSS que produjo una matriz no finita."
        }
        "htmlImport.warn.transform.skew_dropped" => "Se descartó el sesgado del transform CSS.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "Se aproximó un transform CSS con escala cero o no finita."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "Se aproximó el reflejo del transform CSS."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "Se ignoró el desplazamiento Z de transform-origin de CSS."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "Se descartó una escala de transform CSS que no pudo integrarse en el tamaño del nodo."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "Se aproximó la escala de transform CSS integrada en el tamaño del nodo."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "Se ignoró la escala de transform CSS en un elemento de tamaño automático."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "Se aproximó el background-repeat CSS direccional o con espaciado."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "Se ignoró un tamaño explícito de mosaico de fondo CSS."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "Se aproximó background-size de CSS en un elemento de tamaño automático."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "Se aproximó el background-size CSS que necesita el tamaño intrínseco de la imagen."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "Se ignoró un background-position CSS no admitido."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "Se ignoró una URL vacía de imagen de fondo CSS."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => {
            "Se ignoraron los degradados cónicos CSS."
        }
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "Se ignoró una capa de background-image CSS no admitida."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "Se ignoró un color de fondo CSS sin resolver."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "Se ignoró background-position de CSS."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "Se aproximaron los colores de borde CSS por lado."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Se aproximaron los estilos de borde CSS mixtos por lado."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "Se aproximó un estilo de borde CSS complejo."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "Se aproximó un estilo de borde CSS no admitido."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Se aproximaron los radios de borde CSS elípticos."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "Se ignoró un radio de borde CSS no admitido."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "Se ignoró una capa de box-shadow CSS no admitida."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "Se ignoró el método de interpolación de color del degradado CSS."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "Se ignoró una dirección de linear-gradient CSS no admitida."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "Se ignoraron las sugerencias de color de los degradados CSS."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "Se ignoró una parada de color de degradado CSS no admitida."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "Se ignoró un degradado CSS con menos de dos paradas utilizables."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "Se aproximó un degradado CSS repetido."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "Se aproximaron las paradas de degradado CSS fuera de rango."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "Se ignoró un radio de desenfoque CSS no admitido."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "Se ignoró un drop-shadow() de filtro CSS no admitido."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "Se ignoró una función de filtro CSS no admitida."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "Se ignoró una función backdrop-filter CSS no admitida."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "Se ignoró un background-blend-mode CSS no admitido."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "Se aproximó mix-blend-mode de CSS en rellenos individuales."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "Se ignoró un mix-blend-mode CSS no admitido."
        }
        "htmlImport.warn.visual.property_not_representable" => "Se ignoró {{property}} de CSS.",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "Se ignoró background-size de CSS en un degradado."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "Se ignoró una posición de radial-gradient CSS no admitida."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "Se aproximó un radial-gradient CSS elíptico."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "Se aproximó una palabra clave de extensión de radial-gradient CSS."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "Se ignoró un tamaño de radial-gradient CSS no admitido."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "Se ignoró una capa de text-shadow CSS no admitida."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "Se ignoraron las capas de text-shadow CSS posteriores a la primera."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "Se ignoró text-shadow de CSS en un elemento en línea."
        }
        "htmlImport.warn.list.style_image_ignored" => "No se importó list-style-image de CSS.",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "Se aproximó un marcador colgante con `list-style-position: outside`."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "Se aproximó el list-style-type CSS no admitido `{{value}}`."
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "Se aproximó object-fit:scale-down de CSS."
        }
        "htmlImport.warn.media.object_fit_none_ignored" => "Se ignoró object-fit:none de CSS.",
        "htmlImport.warn.media.object_position_ignored" => "Se ignoró object-position de CSS.",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "Se ignoró un mix-blend-mode CSS no admitido en una imagen."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "Un elemento <svg> en línea se importó como marcador de posición."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "Se aproximó un tipo de <input> no admitido."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "El elemento <{{tag}}> se importó como marcador de posición."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "Se aproximó un <picture> cuyos tipos de origen no se pueden descodificar."
        }
        "htmlImport.warn.table.rowspan_ignored" => "No se importó el atributo HTML rowspan.",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "Se aproximaron los anchos de columna de una tabla cuyos grupos de filas no aplanó el CSS."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "Se aproximaron los anchos de columna de una tabla CSS sin ancho definido."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "Se ignoró el <base href> no válido {{href}}."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "Se ignoró el <base href> {{href}} fuera del origen del proyecto."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "La hoja de estilos externa {{url}} no está disponible."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "La imagen {{url}} fuera del origen del proyecto se importó como marcador de posición."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "La imagen no disponible {{url}} se importó como marcador de posición."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "Se ignoró el @import CSS no válido {{prelude}}."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "El @import CSS {{reference}} no está disponible."
        }
        "htmlImport.warn.resource.css_import_cycle" => "Se ignoró el @import CSS cíclico {{url}}.",
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "Se ignoró el @import CSS {{url}} más allá del nivel {{max_depth}}."
        }
        "htmlImport.warn.resource.css_import_unavailable" => {
            "El @import CSS {{url}} no está disponible."
        }
        "htmlImport.warn.project.multiple_html_entries" => {
            "Se encontraron {{count}} entradas HTML; se eligió {{entry}} y se aproximó el resto."
        }
        "htmlImport.warn.snapshot.truncated" => "Se descartó parte de la captura del navegador.",
        "htmlImport.warn.snapshot.node_limit" => {
            "Se alcanzó el límite de nodos; se omitió el resto del contenido de la captura."
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} imágenes contaminadas por CORS, conservadas como URL remotas, no están disponibles."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "Se descartó un nodo de la captura con un rectángulo ausente o no válido."
        }
        "htmlImport.warn.snapshot.unknown_kind" => {
            "Se descartó un nodo de la captura de tipo desconocido."
        }
        "htmlImport.warn.snapshot.rejected" => "Se descartó la captura del navegador ({{reason}}).",
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "Se ignoró una transformación no admitida de la captura."
        }
        "htmlImport.warn.css.media_empty_query" => "Se ignoró una consulta @media vacía.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "Se ignoró el tipo de @media no admitido '{{name}}'."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "Se ignoró la condición de @media no admitida '{{input}}'."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "Se ignoró la orientación de @media no válida '{{value}}'."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "Se ignoró la característica de @media no admitida '{{name}}'."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "Se ignoró el rango de @media no admitido '({{input}})'."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "Se ignoró el rango de @media no válido '({{input}})'."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "Se ignoró la longitud de @media no válida '{{value}}'."
        }
        "htmlImport.diagnostics.title" => "Importación de HTML finalizada",
        "htmlImport.diagnostics.summary" => "Elementos degradados: {{count}}",
        "htmlImport.diagnostics.dismiss" => "Cerrar",
        "htmlImport.diagnostics.expand" => "Mostrar detalles",
        "htmlImport.diagnostics.collapse" => "Ocultar detalles",
        "htmlImport.diagnostics.more" => "+{{count}} más",
        "dialog.pptxTitle" => "Exportar a PowerPoint",
        "dialog.pptxSummary" => "Se exportaron {{count}} diapositivas a:",
        "dialog.pptxEmpty" => "Esta presentación no tiene diapositivas visibles para exportar.",
        "settings.agents.acpQuickAdd" => "Añadir rápido",
        "settings.agents.acpPresetAdd" => "Añadir",
        "settings.agents.acpNotInstalled" => "No instalado",
        "assetCenter.title" => "Centro de recursos",
        "assetCenter.tab.templates" => "Plantillas",
        "assetCenter.tab.styles" => "Estilos",
        "assetCenter.style.empty" => "No hay estilos coincidentes",
        "assetCenter.style.pinned" => "Fijado",
        "assetCenter.style.searchPlaceholder" => "Buscar estilos o etiquetas",
        "assetCenter.style.generateHint" => "Un documento nuevo creado a partir de tu tema, con el estilo fijado.",
        "ai.pinnedStyle" => "Estilo: {{name}}",
        "assetCenter.style.import" => "Importar estilo",
        "assetCenter.style.mine" => "Mis estilos",
        "assetCenter.style.builtIn" => "Estilos integrados",
        "assetCenter.style.importTitle" => "Importar DESIGN.md",
        "assetCenter.style.importHint" => "Pega el DESIGN.md completo y luego confirma la importación.",
        "assetCenter.style.importSource" => "Puedes copiar un estilo de una biblioteca DESIGN.md como styles.refero.design.",
        "assetCenter.style.importConfirm" => "Importar",
        "assetCenter.style.importCancel" => "Cancelar",
        "assetCenter.style.importPickFile" => "Elegir archivo…",
        "assetCenter.style.importHintFile" => "Elige un archivo DESIGN.md o pega el documento completo abajo.",
        "assetCenter.style.importPlaceholder" => "Pega aquí tu DESIGN.md",
        "assetCenter.style.importEmpty" => "Ese archivo está vacío o es demasiado corto para ser una guía de estilo.",
        "assetCenter.style.importNotText" => "Ese archivo no se puede leer como texto Markdown.",
        "assetCenter.style.importTooLarge" => "Ese archivo supera los 512 KB.",
        "slidesPanel.tabSlides" => "Diapositivas",
        "slidesPanel.tabCards" => "Tarjetas",
        "slidesPanel.present" => "Presentar",
        "slidesPanel.exportPdf" => "Exportar PDF",
        "slidesPanel.exportAllSlides" => "Exportar todas las diapositivas",
        "slidesPanel.exportSelectedSlides" => "Exportar diapositivas seleccionadas ({{count}})",
        "settings.tab.ai" => "IA",
        "settings.agents.heroTitle" => "Conecta tu proveedor de IA",
        "settings.agents.heroSubtitle" => "OpenPencil impulsa tus agentes CLI locales y proveedores de API: conecta uno para empezar a generar diseños.",
        "settings.agents.statusConnected" => "Conectado",
        "settings.agents.statusNotConnected" => "Sin conectar",
        "settings.agents.statusChecking" => "Comprobando…",
        "settings.mcp.heroTitle" => "Conecta OpenPencil por MCP desde fuera",
        "settings.mcp.heroSubtitle" => "Apunta cualquier CLI o editor compatible con MCP a este espacio de trabajo y maneja el lienzo con las mismas herramientas del agente integrado.",
        "settings.mcp.terminalFootnote" => "* Al iniciar, MCP se configura automáticamente para las herramientas CLI seleccionadas.",
        "settings.mcp.customConfigTitle" => "Configuración personalizada del servidor MCP",
        "settings.mcp.customConfigDesc" => "Pégalo en cualquier cliente que lea un bloque de servidor MCP estándar.",
        "settings.mcp.copyConfig" => "Copiar config MCP",
        "settings.system.heroTitle" => "Preferencias del sistema",
        "settings.system.heroSubtitle" => "Apariencia, actualizaciones y comportamiento del lienzo en esta instalación.",
        "settings.system.appearance" => "Apariencia",
        "settings.system.appearanceLight" => "Claro",
        "settings.system.appearanceDark" => "Oscuro",
        "settings.system.pencilCursor" => "Cursor de lápiz",
        "settings.images.heroTitle" => "Imágenes para tus diseños",
        "settings.images.heroSubtitle" => "Busca fotos en Openverse o conecta un proveedor para generarlas cuando las necesites.",
        "settings.fonts.heroTitle" => "Fuentes de este documento",
        "settings.fonts.heroSubtitle" => "Resuelve las fuentes que pide un documento y no están en este equipo, y gestiona las que importaste.",
        "settings.account.heroTitle" => "Tu cuenta",
        "settings.account.heroSubtitle" => "Inicia sesión para sincronizar tu espacio de trabajo y tu licencia entre dispositivos.",
        "tooltip.topbar.file" => "Archivo",
        "tooltip.topbar.import" => "Importar",
        "tooltip.topbar.language" => "Idioma",
        "tooltip.topbar.collaboration" => "Colaboración",
        "tooltip.topbar.preview" => "Vista previa",
        "tooltip.topbar.exitPreview" => "Salir de la vista previa",
        "tooltip.topbar.account" => "Cuenta",
        "settings.agents.providerRollMore" => "y {{count}} más",
        "ai.thinking.adaptive" => "Razonamiento: automático",
        "ai.thinking.disabled" => "Razonamiento: desactivado",
        "ai.thinking.enabled" => "Razonamiento: activado",
        "ai.designProgress.detail.repairsApplied" => "{{count}} reparación(es) automática(s) aplicada(s)",
        "ai.designProgress.detail.repairsMore" => "… y {{count}} más (ver el registro)",
        "ai.styleCard.builtin" => "Estilo integrado",
        "ai.styleCard.imported" => "DESIGN.md importado",
        "ai.styleCard.documentDesignMd" => "design.md del documento",
        _ => return super::es_collab::lookup(key),
    })
}
