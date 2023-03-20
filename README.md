# SideStore Downloader

[![Rust](https://github.com/SideStore/sidestore_downloader/actions/workflows/rust.yml/badge.svg)](https://github.com/SideStore/sidestore_downloader/actions/workflows/rust.yml)

![Alt](https://repobeats.axiom.co/api/embed/4fa23f774203cbc3d7a55f413aef46cd62c3201a.svg "Repobeats analytics image")

## Uses

1. Download the latest [SideStore](https://sidestore.io) .ipa file
2. Generate and embed `iDevice` pairing files into the `.ipa`'s `Info.plist` file
3. Let users choose which anisette server to use by also embedding that url in the Info.plist of the .ipa

## FAQ

Q: __Why does this project exist? Aren't their easier ways to specify/import pairing files and anisette server urls?__

A: Yes, there probably are easier ways, however, none of us know how to navigate AltStore's very confusing project structure. If you would like to help us add ways to specify both the pairing file and anisette URL at runtime, contributions are welcome!
