# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 모바일 프로젝트 준비 (clone 후 최초 1회)

`src-tauri/gen/`(Xcode·Android 네이티브 프로젝트)은 전부 생성물이라 저장소에
커밋하지 않습니다. clone 후 빌드 전에 아래로 재생성합니다.

```bash
bun -F mobile tauri ios init       # iOS — xcodegen, cocoapods 필요
bun -F mobile tauri android init   # Android — Android SDK/NDK 필요
```

소스오브트루스는 [`src-tauri/tauri.conf.json`](./src-tauri/tauri.conf.json)과
[`src-tauri/icons/`](./src-tauri/icons/)이며, init이 이들로부터 `gen/`을 만듭니다.

## iOS 코드 서명

iOS 실제 빌드/배포(`tauri ios build`, `tauri ios run`)는 Apple 코드 서명이 필요합니다. 서명에 쓸 Apple 개발자 **팀 ID**(10자리, 조직 단위 공유 값)를 [`src-tauri/tauri.conf.json`](./src-tauri/tauri.conf.json)의
`bundle.iOS.developmentTeam`에 넣습니다. 현재는 빈 값이며, 팀 ID가 정해지면
이 필드에 작성합니다.

- 시뮬레이터 빌드(`tauri ios dev`)는 서명이 필요 없어 공란으로 두었습니다.
- 팀 ID 확인: Xcode → Settings → Accounts → 계정 선택 → 팀 목록의 10자리 ID,
  또는 <https://developer.apple.com> → Membership → Team ID.
- 임시로 덮어쓰려면 `APPLE_DEVELOPMENT_TEAM` 환경변수를 설정하면 됩니다.
