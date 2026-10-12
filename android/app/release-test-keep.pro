# Applied to the app only for -Plibrespydus.testBuildType=release. The test APK resolves shared
# libraries (androidx.tracing, the Kotlin stdlib, kotlinx.coroutines) from the app APK, where R8
# would strip whatever the app itself doesn't use. The app's own code, JNA and UniFFI stay
# shrunk exactly as in a normal release build.
-keep class androidx.tracing.** { *; }
-keep class kotlin.** { *; }
-keep class kotlinx.coroutines.** { *; }
