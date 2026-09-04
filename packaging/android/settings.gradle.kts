// The canonical repos are listed first; Aliyun mirrors follow as
// fallbacks for environments where dl.google.com is not reachable.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
        maven("https://maven.aliyun.com/repository/google")
        maven("https://maven.aliyun.com/repository/gradle-plugin")
        maven("https://maven.aliyun.com/repository/public")
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
        maven("https://maven.aliyun.com/repository/google")
        maven("https://maven.aliyun.com/repository/public")
        // Douyin OpenSDK (native sign-in) is published only on ByteDance's
        // own repository.
        maven("https://artifact.bytedance.com/repository/AwemeOpenSDK")
    }
}

rootProject.name = "OpenPencilPlayer"
include(":app")
