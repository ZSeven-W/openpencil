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
        "sceneTemplate.filter.all" => "Todo",
        "sceneTemplate.scene.tutorial" => "Tutorial",
        "sceneTemplate.scene.comparison" => "Comparativa",
        "sceneTemplate.scene.carousel" => "Tarjetas de conocimiento",
        "sceneTemplate.scene.slides" => "PPT",
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
        "fileMenu.newFromTemplate" => "Nuevo a partir de una plantilla",
        _ => return super::es_collab::lookup(key),
    })
}
