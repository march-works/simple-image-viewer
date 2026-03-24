# release スキル

このスキルは、Simple Image Viewer のリリース作業を完全自動化するためのワークフローを定義します。
バージョン更新から Gist 更新まで、すべての手順をスキル 1 回の呼び出しで完結させます。

## 起動条件

以下のような依頼を受けたときに、このスキルを使用する:

- 「リリースして」
- 「バージョンを上げて」
- 「vup して」
- 「release を出して」

## 依存ツール

- `git` CLI（必須）
- `gh` CLI（必須）

スキル起動時に最初に認証状態を確認する:

```bash
gh auth status
```

認証されていない場合はユーザーに `gh auth login` を実行するよう案内して終了する。

## ワークフロー

### Step 1: 現在のバージョン確認

`package.json` を読み取って現在のバージョンを表示し、ユーザーに新しいバージョン番号を確認する。

```
現在のバージョン: X.Y.Z
新しいバージョン番号を入力してください:
```

確認が取れるまで次のステップには進まない。

### Step 2: バージョン更新

以下の 2 ファイルのバージョンを更新する（Edit ツールを使用）:

| ファイル | フィールド |
| --- | --- |
| `package.json` | `"version": "X.Y.Z"` |
| `src-tauri/tauri.conf.json` | `"version": "X.Y.Z"` |

### Step 3: コミット & Push（master）

```bash
git add package.json src-tauri/tauri.conf.json
git commit -m "vup to vX.Y.Z"
git push origin master
```

### Step 4: release ブランチへマージ

```bash
git checkout release
git merge master
git push origin release
git checkout master
```

### Step 5: GitHub Actions の完了を待機

新バージョンのタグ付きドラフトリリースが作成され、`latest.json` アセットが含まれるまでポーリングする。
最大 30 分（30 秒ごとに最大 60 回）待機する。

```bash
VERSION="vX.Y.Z"
REPO="march-works/simple-image-viewer"
release_id=""

for i in $(seq 1 60); do
  release_id=$(gh api repos/$REPO/releases \
    --jq ".[] | select(.draft == true and .tag_name == \"$VERSION\") | .id")
  if [ -n "$release_id" ]; then
    has_latest=$(gh api repos/$REPO/releases/$release_id \
      --jq '.assets[] | select(.name == "latest.json") | .name')
    if [ -n "$has_latest" ]; then
      echo "Build complete! release_id=$release_id"
      break
    fi
  fi
  echo "Waiting for GitHub Actions... ($i/60)"
  sleep 30
done

if [ -z "$release_id" ]; then
  echo "Timeout: GitHub Actions did not complete within 30 minutes."
  exit 1
fi
```

### Step 6: Draft を解除して正式リリース

```bash
gh api --method PATCH repos/march-works/simple-image-viewer/releases/$release_id \
  -f draft=false
```

### Step 7: Gist を latest.json で更新

```bash
# latest.json のダウンロード URL を取得
url=$(gh api repos/march-works/simple-image-viewer/releases/$release_id \
  --jq '.assets[] | select(.name == "latest.json") | .browser_download_url')

# latest.json の内容を取得
content=$(curl -sL "$url")

# Gist のファイル名を取得
filename=$(gh api /gists/e7c9399157590f0fb28882ff7cbe31dd \
  --jq '.files | keys[0]')

# Gist を更新
gh api --method PATCH /gists/e7c9399157590f0fb28882ff7cbe31dd \
  -f "files[$filename][content]=$content"

echo "Gist updated successfully!"
```

### 完了メッセージ

すべてのステップが完了したら以下を表示する:

```
リリース vX.Y.Z が完了しました。

- package.json / tauri.conf.json のバージョンを更新
- master ブランチを push
- release ブランチへマージ & push
- GitHub Actions によるビルドが完了
- GitHub Releases の Draft を解除
- Gist (e7c9399157590f0fb28882ff7cbe31dd) を最新の latest.json で更新
```

## 注意事項

- Step 1 でユーザーの確認が取れるまでコミット等の操作を行わない
- Step 5 のポーリングが失敗した場合（タイムアウト）は途中経過をユーザーに報告し、手動での確認を案内する
- `gh` CLI が未認証の場合は `gh auth login` を案内して終了する
