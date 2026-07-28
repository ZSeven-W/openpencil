//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `en_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Search images...",
        "imagePanel.searching" => "Searching...",
        "imagePanel.noResults" => "No results found",
        "imagePanel.searchPrompt" => "Search for images",
        "imagePanel.sourceNotice" => {
            "Images from {{source}}. Freely licensed — verify license before use."
        }
        "imagePanel.genNotConfigured" => "Image generation not configured",
        "imagePanel.openSettings" => "Open Settings",
        "imagePanel.promptPlaceholder" => "Describe the image...",
        "providerProbe.connectedViaCli" => "Connected via {{name}} CLI",
        "providerProbe.cliExitedWithError" => "{{name}} CLI exited with an error",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI produced no version output",
        "providerProbe.modelQueryFailed" => "{{name}} model query failed or timed out",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} model query failed. Run {{command}} once to authenticate."
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} model query requires authentication. Run {{command}} once to sign in."
        }
        "providerProbe.unrecognizedModelCatalog" => {
            "{{name}} returned an unrecognized model catalog"
        }
        "providerProbe.connectedAs" => "Connected as @{{login}}{{method}}",
        "providerProbe.connectedViaGithub" => "Connected via GitHub",
        "importProgress.figmaTitle" => "Parsing Figma file…",
        "importProgress.htmlTitle" => "Parsing HTML and page resources…",
        "importProgress.htmlSubtitle" => "Loading styles and images. Please wait.",
        "importProgress.largeFileSubtitle" => "Large files take a few seconds. Please wait.",
        "account.signedOutHint" => "Sign in to sync your settings and preferences",
        "code.noUsableCode" => "The AI returned no usable code. Retry or switch AI models.",
        "code.previousResultKept" => "The previous generated result is still available",
        _ => return super::en_collab::lookup(key),
    })
}
