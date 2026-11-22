# screen

screen lets one person help another person use their computer. The person at that computer must click **Accept** before anyone can see or control their screen.

## Download

Click the button for your computer. The file downloads immediately.

<p align="center">
  <a href="https://github.com/Yorxsm/screen/releases/download/desktop-0.1.2/rustdesk-1.5.3-x86_64.msi"><img alt="Download the Windows installer" src="https://img.shields.io/badge/Download-Windows%20installer-0078D4?style=for-the-badge&logo=windows&logoColor=white"></a>
</p>

<p align="center">
  <a href="https://github.com/Yorxsm/screen/releases/download/desktop-0.1.2/rustdesk-1.5.3-aarch64.dmg"><img alt="Download for Mac Apple Silicon" src="https://img.shields.io/badge/Download-Mac%20Apple%20Silicon%20%28M1%2FM2%2FM3%2FM4%29-000000?style=for-the-badge&logo=apple&logoColor=white"></a>
</p>

<p align="center">
  <a href="https://github.com/Yorxsm/screen/releases/download/desktop-0.1.2/rustdesk-1.5.3-x86_64.dmg"><img alt="Download for Mac Intel" src="https://img.shields.io/badge/Download-Mac%20Intel-555555?style=for-the-badge&logo=apple&logoColor=white"></a>
</p>

Apple Silicon is for M1, M2, M3, and M4 Macs. Mac Intel is for older Macs.

Other files:

- [Windows portable program (.exe)](https://github.com/Yorxsm/screen/releases/download/desktop-0.1.2/rustdesk-1.5.3-x86_64.exe), if you do not want the installer
- [All files for this version](https://github.com/Yorxsm/screen/releases/tag/desktop-0.1.2)

### If your computer warns you

These builds are unsigned, so Windows and Mac may ask an extra question before they open.

- **Windows.** If SmartScreen appears, click **More info**, then **Run anyway**.
- **Mac.** Right-click the app and choose **Open**. If the Mac says the app is damaged, open Terminal and run `xattr -dr com.apple.quarantine /Applications/<app>.app`, then open the app again.

## How it works

1. Staff open the invite link. screen opens on their computer.
2. Their computer asks whether someone may control the screen. Staff must click **Accept**.
3. Only after that click can the other person see or control the screen. **Cancel**, or closing the prompt, stops it. The link does not click Accept for them, and it does not turn on unattended access.

## License

screen is licensed under AGPL-3.0 and is derived from open-source software. See [LICENSE](LICENSE).
