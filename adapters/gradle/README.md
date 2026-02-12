# Gradle adapter

`adapters/gradle/libforge-gradle` is where a Kotlin/Gradle plugin will live, but it currently contains only placeholder files (`build.gradle.kts` and `LibForgePlugin.kt`). There is no published Gradle artifact yet. When this module gains a real implementation it will read `libforge.yaml` (cache `precompiled_binaries`, validate `build_id`, etc.) and expose tasks that download the signed manifest/artifacts or bundle them into Android/AAR components.
