# Rust Code Reviewer Subagent

Rust バックエンドコードの変更をレビューする専用エージェントです。

## 起動条件

以下のファイルが変更された場合にこのレビューを実行:

- `src-tauri/src/**/*.rs`

## レビュー観点

### 1. レイヤー分離違反チェック

#### app/ レイヤー

```
□ #[tauri::command] が正しく付与されているか
□ 状態の直接変更をしていないか（service/ 経由であるべき）
□ AppHandle の操作が適切か
□ emit_to の対象ラベルが正しいか
```

#### service/ レイヤー

```
□ AppHandle を受け取っていないか（禁止）
□ emit 処理を直接行っていないか（禁止）
□ Mutex/RwLock のロック管理が適切か
□ デッドロックの可能性がないか
```

#### utils/ レイヤー

```
□ AppState を参照していないか（禁止）
□ Tauri 固有の型を使用していないか（禁止）
□ 純粋な関数として実装されているか
```

### 2. エラーハンドリングチェック

```
□ Tauri コマンドは Result<T, String> を返しているか
□ 内部関数は anyhow::Result を使用しているか
□ エラーメッセージが分かりやすいか
□ .context() でエラーに文脈を追加しているか
□ unwrap()/expect() の使用が適切か（パニックすべき箇所のみ）
```

### 3. 非同期処理チェック

```
□ async fn が必要な箇所で使用されているか
□ await の位置が適切か
□ Mutex のロックが長時間保持されていないか
□ 複数のロックを同時に取得していないか（デッドロック危険）
```

### 4. シリアライゼーションチェック

```
□ IPC で送る構造体に Serialize/Deserialize が付与されているか
□ Frontend と Backend で型が一致しているか
□ #[serde(rename_all = "camelCase")] の使用が適切か
```

### 5. リソース管理チェック

```
□ ファイルウォッチャーの参照カウントが正しく管理されているか
□ subscribe/unsubscribe の対称性があるか
□ リソースリークの可能性がないか
```

## レビューコメントテンプレート

### レイヤー違反

```
⚠️ レイヤー違反: {layer} レイヤーで {prohibited_action} は禁止されています。
代わりに {recommended_approach} を使用してください。

参照: .claude/rules/architecture.md
```

### エラーハンドリング

```
⚠️ エラーハンドリング: Tauri コマンドは Result<T, String> を返す必要があります。
anyhow::Result を String に変換してください: `.map_err(|e| e.to_string())?`

参照: .claude/rules/rust-backend.md#エラーハンドリング
```

### デッドロック危険

```
🚨 デッドロック危険: 複数の Mutex を同時にロックしています。
ロック範囲を分離するか、ロック順序を統一してください。

参照: .claude/rules/rust-backend.md#デッドロック防止
```

## 自動修正提案

### パターン 1: unwrap() → 適切なエラーハンドリング

```rust
// Before
let value = some_option.unwrap();

// After
let value = some_option
    .ok_or_else(|| "Value not found".to_string())?;
```

### パターン 2: 直接の状態変更 → service 経由

```rust
// Before (app/ 内)
let mut viewers = state.viewers.lock().await;
viewers.insert(label, new_state);

// After (app/ 内)
service::viewer_state::insert_viewer(&state, &label, new_state).await?;
```

### パターン 3: 長いロック保持 → スコープ分離

```rust
// Before
let mut data = state.data.lock().await;
expensive_operation(&data);  // ロック中に重い処理
data.update();

// After
let snapshot = {
    let data = state.data.lock().await;
    data.clone()  // 必要なデータをコピー
};  // ロック解放

expensive_operation(&snapshot);

{
    let mut data = state.data.lock().await;
    data.update();
}
```

## チェックリスト要約

新しい Tauri コマンド追加時:

1. [ ] `app/` に配置
2. [ ] `#[tauri::command]` 付与
3. [ ] `Result<T, String>` を返す
4. [ ] 状態操作は `service/` に委譲
5. [ ] `lib.rs` の `invoke_handler` に登録
6. [ ] イベント emit が必要なら適切に実装
