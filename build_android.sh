# Create environment variable to `OPENSSL_DIR`
export OPENSSL_DIR=$(realpath "dependencies/openssl/openssl-android-arm64-v8a")

# This is very important so that we dont have to specify any features
cd mobile

# Compile the application
cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  build #--release

# Build APK
./gradlew build

# Install APK to running emulator
./gradlew installDebug

echo Application started

# Start installed aplication in the emulator
adb shell am start -n co.realfit.agdkeframe/.MainActivity

echo Debug started

# Start logging the debug output
adb logcat