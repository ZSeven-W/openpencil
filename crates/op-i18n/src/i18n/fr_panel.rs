//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `fr_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Rechercher des images…",
        "imagePanel.searching" => "Recherche…",
        "imagePanel.noResults" => "Aucun résultat",
        "imagePanel.searchPrompt" => "Recherchez des images",
        "imagePanel.sourceNotice" => {
            "Images de {{source}}. Licence libre — vérifiez la licence avant utilisation."
        }
        "imagePanel.genNotConfigured" => "Génération d'images non configurée",
        "imagePanel.openSettings" => "Ouvrir les réglages",
        "imagePanel.promptPlaceholder" => "Décrivez l'image…",
        "providerProbe.connectedViaCli" => "Connecté via la CLI {{name}}",
        "providerProbe.cliExitedWithError" => "La CLI {{name}} s'est arrêtée sur une erreur",
        "providerProbe.cliNoVersionOutput" => "La CLI {{name}} n'a produit aucune information de version",
        "providerProbe.modelQueryFailed" => "La requête de modèles {{name}} a échoué ou expiré",
        "providerProbe.modelQueryFailedRunLogin" => "La requête de modèles {{name}} a échoué. Exécutez {{command}} une fois pour vous authentifier.",
        "providerProbe.modelQueryNeedsAuth" => "La requête de modèles {{name}} nécessite une authentification. Exécutez {{command}} une fois pour vous connecter.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} a renvoyé un catalogue de modèles non reconnu",
        _ => return super::fr_collab::lookup(key),
    })
}
