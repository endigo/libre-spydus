// Top-level build file. AGP 9 compiles Kotlin itself (built-in Kotlin); the Kotlin plugin is
// declared only to pin the Kotlin Gradle plugin version the compose/serialization plugins match.
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.kotlin.serialization) apply false
}
