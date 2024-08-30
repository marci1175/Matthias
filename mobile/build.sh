
echo $OPENCV_INCLUDE_PATHS
echo $OPENSSL_DIR

read

echo "Building Android application. . . "

ANDROID_HOME="C:\Users\marci\AppData\Local\Android\Sdk"
NDK_HOME="$ANDROID_HOME/ndk/27.0.12077973"

APK_TARGET_DIR="target/apk"
RUST_TARGET="x86_64-linux-android"
ARCH="armeabi-v7a"

cargo build --release --target x86_64-linux-android

rm -rf $APK_TARGET_DIR/compiled $APK_TARGET_DIR/root/lib

mkdir -p $APK_TARGET_DIR/root/lib/$ARCH
cp target/$RUST_TARGET/release/android-test $APK_TARGET_DIR/root/lib/$ARCH/

mkdir -p $APK_TARGET_DIR/compiled

rm -rf $APK_TARGET_DIR/compiled/*

for i in $(find android/res -type f);
do 
    # echo $(realpath $i)
    aapt2 compile -o $APK_TARGET_DIR/compiled $(realpath $i)
done

aapt2 link -o $APK_TARGET_DIR/apk.apk -I $ANDROID_HOME/platforms/android-34/android.jar --manifest android/AndroidManifest.xml $APK_TARGET_DIR/compiled/*

APK_TARGET_DIR_FULL_PATH=$(realpath $APK_TARGET_DIR)

cd $APK_TARGET_DIR/root

7z a -tzip $APK_TARGET_DIR_FULL_PATH/apk.apk *.so -mx=9 -r -spf

$ANDROID_HOME/build-tools/34.0.0/zipalign.exe -p -f -v 4 $APK_TARGET_DIR_FULL_PATH/apk.apk $APK_TARGET_DIR_FULL_PATH/apk-aligned.apk

cd -

# generate keystore with
# keytool -genkey -v -keystore test.keystore -alias test -keyalg RSA -keysize 2048 -validity 10000
$ANDROID_HOME/build-tools/34.0.0/apksigner.bat sign --ks test.keystore --ks-pass pass:test00 --min-sdk-version 34 --out $APK_TARGET_DIR/apk-signed.apk $APK_TARGET_DIR/apk-aligned.apk

read