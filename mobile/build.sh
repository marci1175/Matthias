cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  build
./gradlew build
./gradlew installDebug

read