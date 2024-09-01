export OPENSSL_DIR=$(realpath "deps/openssl-android-arm64-v8a")

cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  build
./gradlew build
./gradlew installDebug

echo Application started

adb shell am start -n co.realfit.agdkeframe/.MainActivity

echo Debug started

adb logcat
