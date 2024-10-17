# Create directories
mkdir production-builds -p
mkdir production-builds/desktop -p
mkdir production-builds/mobile -p

# Create environment variable to `OPENSSL_DIR`
export OPENSSL_DIR=$(realpath "dependencies/openssl/openssl-android-arm64-v8a")

# This is very important so that we dont have to specify any features
cd mobile

echo "Compile android application"

# Compile the application
cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  build --release

echo "Building android application"

# Build APK
./gradlew build

echo "Assembling release android build"

# Install APK to running emulator
./gradlew assembleRelease

echo "Copying release files to release folder"

# Copy release files
cp app/build/outputs/apk/release/app-release-unsigned.apk ../production-builds/mobile

# Go to the desktop folder
cd ../desktop

echo Compiling desktop application into production-builds/desktop

# Build project
cargo b --release --target-dir ../production-builds/desktop

echo "Finished building all the projects."

read