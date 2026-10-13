# JNA: native code looks classes, fields and methods up by name; Structure subclasses are read
# reflectively in declared field order.
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
-dontwarn java.awt.**
-dontwarn com.sun.jna.**

# UniFFI bindings (uniffi.librespydus_core): JNA-registered native methods, Structure/Callback
# implementations and the FfiConverters are all reached by reflection from JNA.
-keep class uniffi.librespydus_core.** { *; }
-keep interface uniffi.librespydus_core.** { *; }

# Type-safe navigation routes and the persisted favourites/library JSON.
-keep @kotlinx.serialization.Serializable class com.librespydus.app.** { *; }
