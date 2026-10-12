plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
}

// ABIs with a built core library in src/main/jniLibs (see scripts/build-core.sh).
val coreAbis: List<String> = file("src/main/jniLibs").listFiles()
    ?.filter { File(it, "liblibrespydus_core.so").isFile }
    ?.map { it.name }
    ?.sorted()
    .orEmpty()
if (coreAbis.isEmpty()) {
    logger.warn("w: no liblibrespydus_core.so in app/src/main/jniLibs — run android/scripts/build-core.sh before building an APK")
}

android {
    namespace = "com.librespydus.app"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.librespydus.app"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        // Package only the ABIs scripts/build-core.sh produced: JNA ships libjnidispatch for many
        // more, and a device must never pick an ABI folder that lacks the core.
        ndk { abiFilters += coreAbis.ifEmpty { listOf("arm64-v8a", "x86_64") } }
    }

    // `-Plibrespydus.testBuildType=release` runs the instrumented tests against the minified
    // release build (checks the R8 keep rules for JNA + UniFFI on a device).
    testBuildType = providers.gradleProperty("librespydus.testBuildType").getOrElse("debug")
    val minifiedTests = testBuildType == "release"

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            testProguardFiles("test-proguard-rules.pro")
            if (minifiedTests) proguardFiles("release-test-keep.pro")
            // No release keystore yet: sign with the debug key so a release build can be installed
            // locally to check R8. Replace with a real signingConfig before distributing.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    // The UniFFI bindings are compiled straight from the core's checked-in output (single source
    // of truth, never copied). The matching .so files come from scripts/build-core.sh.
    sourceSets.named("main") {
        kotlin.directories += rootProject.file("../core/generated/kotlin").path
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    testOptions {
        unitTests.isReturnDefaultValues = true
    }

    packaging {
        resources.excludes += "/META-INF/{AL2.0,LGPL2.1}"
    }
}

kotlin {
    jvmToolchain(17)
}

dependencies {
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.compose.foundation)
    implementation(libs.androidx.compose.material3)
    debugImplementation(libs.androidx.compose.ui.tooling)

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.navigation.compose)
    implementation(libs.androidx.datastore.preferences)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.coil.compose)
    implementation(libs.coil.network.okhttp)
    // The AAR carries libjnidispatch.so for every Android ABI (the plain jar has desktop ones).
    implementation(libs.jna) { artifact { type = "aar" } }

    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)

    androidTestImplementation(libs.androidx.test.core)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.kotlinx.coroutines.test)
}
