# R8 rules for the instrumented-test APK when it runs against the minified release build
# (-Plibrespydus.testBuildType=release). androidx.test references compile-only annotations.
-dontwarn com.google.errorprone.annotations.**
