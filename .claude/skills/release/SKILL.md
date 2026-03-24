# release スキル

このスキルは、Simple Image Viewer のリリース作業を完全自動化するためのワークフローを定義します。
バージョン更新から Gist 更新まで、すべての手順をスキル 1 回の呼び出しで完結させます。

## 起動条件

バージョン番号を更新してリリース作業を行う意図の依頼を受けたとき。
具体的な表現・粒度（patch/minor/major）・言語は問わない。

例: 「リリースして」「バージョン上げて」「vup して」「パッチ/マイナー/メジャーバージョン上げて」「release を出して」など

## 依存ツール

- `git` CLI（必須）
- `gh` CLI（必須）

スキル起動時に最初に `gh` コマンドが使えるか確認する:

```bash
which gh || echo "not found"
```

見つからない場合は `~/bin/gh` にシンボリックリンクを作成するよう案内する:

```bash
ln -sf "/c/Program Files/GitHub CLI/gh.exe" ~/bin/gh
```

それでも解決しない場合はユーザーに手動での対応を依頼して終了する。

次に認証状態を確認する:

```bash
gh auth status
```

認証されていない場合はユーザーに `gh auth login` を実行するよう案内して終了する。

## ワークフロー

### Step 1: 現在のバージョン確認

`package.json` を読み取って現在のバージョンを表示する。
依頼の内容から bump の粒度（patch/minor/major）が読み取れる場合は次のバージョンを自動提案し、ユーザーに確認を求める。
粒度が不明な場合は入力を促す。

```
現在のバージョン: X.Y.Z
→ 新しいバージョン: X.Y.Z+1 でよいですか？（違う場合は番号を入力）
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
git pull origin release
git merge master
git push origin release
git checkout master
```

### Step 5: GitHub Actions の完了を待機

release ブランチへの push で起動したワークフローの完了を `gh run view` でポーリングする。
`latest.json` の存在チェックは macOS ジョブが先行してアップロードした時点で誤検知するため使用しない。
最大 30 分（30 秒ごとに最大 60 回）待機する。

```bash
VERSION="vX.Y.Z"
REPO="march-works/simple-image-viewer"

# push 直後は run がまだ登録されていない場合があるため少し待つ
sleep 15

# release ブランチの最新 run ID を取得
run_id=""
for i in $(seq 1 10); do
  run_id=$(gh run list --repo $REPO --branch release --limit 1 --json databaseId --jq '.[0].databaseId')
  if [ -n "$run_id" ]; then
    echo "Found run: $run_id"
    break
  fi
  echo "Waiting for run to appear... ($i/10)"
  sleep 10
done

if [ -z "$run_id" ]; then
  echo "Error: Could not find Actions run."
  exit 1
fi

# ワークフローの完了を待機
build_done=""
for i in $(seq 1 60); do
  conclusion=$(gh run view $run_id --repo $REPO --json status,conclusion \
    --jq 'if .status == "completed" then .conclusion else "pending" end')
  if [ "$conclusion" = "success" ]; then
    build_done="true"
    echo "GitHub Actions completed successfully!"
    break
  elif [ "$conclusion" != "pending" ] && [ "$conclusion" != "in_progress" ]; then
    echo "GitHub Actions failed: $conclusion"
    exit 1
  fi
  echo "Waiting for GitHub Actions... ($i/60) [$(gh run view $run_id --repo $REPO --json status --jq '.status')]"
  sleep 30
done

if [ -z "$build_done" ]; then
  echo "Timeout: GitHub Actions did not complete within 30 minutes."
  exit 1
fi

# ドラフトリリースの ID を取得
release_id=$(gh api repos/$REPO/releases \
  --jq ".[] | select(.draft == true and .tag_name == \"$VERSION\") | .id")

if [ -z "$release_id" ]; then
  echo "Error: Could not find draft release for $VERSION"
  exit 1
fi

echo "Build complete! release_id=$release_id"
```

### Step 6: Draft を解除して正式リリース

```bash
gh api --method PATCH repos/march-works/simple-image-viewer/releases/$release_id \
  -f draft=false \
  -f make_latest=true
```

### Step 7: Gist を latest.json で更新

```bash
# latest.json のダウンロード URL を取得
url=$(gh api repos/march-works/simple-image-viewer/releases/$release_id \
  --jq '.assets[] | select(.name == "latest.json") | .browser_download_url')

# latest.json の内容を取得
content=$(curl -sL "$url")

# Gist のファイル名を取得（先頭スラッシュなし: Git Bash のパス変換を回避）
filename=$(gh api gists/e7c9399157590f0fb28882ff7cbe31dd \
  --jq '.files | keys[0]')

# Gist を更新
gh api --method PATCH gists/e7c9399157590f0fb28882ff7cbe31dd \
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
