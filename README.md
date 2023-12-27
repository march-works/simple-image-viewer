# Tauri + Solid + Typescript

This template should help get you started developing with Tauri, Solid and Typescript in Vite.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## リリース手順
1. masterブランチの`package.json`と`src-tauri/tauri.conf.json`のバージョンを更新する
1. masterブランチをpushする
1. master -> releaseにマージする
1. (github actionsでビルド＆リリース完了後)releasesからDraftを外す
1. `https://gist.github.com/march101348/e7c9399157590f0fb28882ff7cbe31dd`を編集
    - versionを`package.json`と同じ数値にする
    - `darwin-x86_64`と`windows-x86_64`の`signature`をそれぞれ対応した.sigファイルの中身の文字列で更新する
    - `url`をリリースされたファイルのURLで更新する