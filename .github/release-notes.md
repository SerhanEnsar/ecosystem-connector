## Install

| Platform | File | How |
|---|---|---|
| Android phone or tablet | `EcosystemConnector-{{VERSION}}-android-arm64-v8a.apk` (most devices) or `-universal.apk` | Open the file on the device and allow installing from this source. Later versions update in place. |
| macOS | `EcosystemConnector-{{VERSION}}-macos-universal.zip` | Unzip and move the app to Applications. The app is not notarized yet: the first time, right-click it and choose **Open**. |
| Windows | `EcosystemConnector-{{VERSION}}-windows-x64.zip` | Unzip anywhere and run `EcosystemConnector.exe`. SmartScreen may warn because the app is not code-signed yet. |
| Command line | `ec-cli-{{VERSION}}-<platform>` | Unpack and run `ec --help`. |

## Verify

Every file has a SHA-256 checksum in `SHA256SUMS.txt` and a build provenance
attestation that proves it was built by this repository's release workflow:

```sh
gh attestation verify EcosystemConnector-{{VERSION}}-android-arm64-v8a.apk -R SerhanEnsar/ecosystem-connector
```
