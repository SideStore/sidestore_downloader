echo "Jackson is the best, he wrote this script and it is awesome. Enjoy it."
# If you hate this script, you hate Jackson
# Just a friendly reminder that Jackson is self-taught and had to Google everything

mkdir release

###########
# Binbows #
###########
echo "Building Michaelsoft Binbows version, I'm so sorry for everyone using Binbows. Must be a struggle."
cargo build --target x86_64-pc-windows-gnu --release
zip -j release/sidestore-downloader-windows.zip target/x86_64-pc-windows-gnu/release/sidestore_downloader.exe

#########
# MacOS #
#########
# Building for MacOS requires MacOS because Apple's linker is proprietary
if [[ $(uname -s) == 'Darwin' ]]; then
    # Build the binaries
    echo "Building MacOS version, the OS given to us by the gods."
    cargo build --target x86_64-apple-darwin --release
    cargo build --target aarch64-apple-darwin --release
    # Combine the binaries into a fat binary
    lipo -create -output sidestore-downloader-macos target/x86_64-apple-darwin/release/sidestore_downloader target/aarch64-apple-darwin/release/sidestore_downloader
    codesign -s - -f sidestore-downloader-macos
    mkdir -p sidestore-downloader
    mv sidestore-downloader-macos sidestore-downloader/sidestore-downloader
    # Place into a DMG to keep 
    hdiutil create -volname sidestore-downloader -srcfolder sidestore-downloader -ov -format UDZO release/sidestore-downloader-macos.dmg
    rm -rf sidestore-downloader
fi

#########
# Linux #
#########
echo "Building Penguin version"
cargo build --target x86_64-unknown-linux-gnu --release
zip -j release/sidestore-downloader-linux.zip target/x86_64-unknown-linux-gnu/release/sidestore_downloader