# gRPC から tauri-plugin-single-instance への移行計画

## 概要

現在の gRPC ベースのシングルトン実装を `tauri-plugin-single-instance` に置き換え、コードの複雑さを大幅に削減します。

## 現状の問題点

### 現在の gRPC 実装

- **約200行のカスタムコード**
- **6以上の依存クレート**: `tonic`, `prost`, `tonic-build`, `sysinfo`, `tokio-stream`, `stream-cancel`
- **Proto ファイルのコンパイル**: build.rs での `tonic_build::compile_protos()`
- **ポート50052のハードコード**: 他アプリとの競合リスク
- **IPv6依存**: `[::1]:50052` の使用

### 削除対象ファイル

```
src-tauri/
├── proto/
│   ├── add_tab.proto      # 削除
│   └── new_window.proto   # 削除
└── src/
    ├── grpc/
    │   ├── mod.rs         # 削除
    │   ├── add_tab.rs     # 削除
    │   ├── new_window.rs  # 削除
    │   └── server.rs      # 削除
    └── lib.rs             # tonic::include_proto! を削除
```

## 移行後のアーキテクチャ

### tauri-plugin-single-instance の導入

```rust
// src-tauri/src/lib.rs
tauri::Builder::default()
    .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
        // 2番目以降のインスタンスが起動しようとした時にコールバック
        // args: コマンドライン引数
        // cwd: 起動時のカレントディレクトリ
    }))
```

### 移行後のフロー

```
2番目のインスタンス起動
    │
    └── OS ネイティブのロック機構により検出
            │
            └── 1番目のインスタンスのコールバックが実行される
                    │
                    ├── ファイルパス引数あり → タブ追加 + ウィンドウフォーカス
                    │
                    └── ファイルパス引数なし → 新規ウィンドウ作成
```

## 実装手順

### 1. プラグインの追加

**Cargo.toml** に追加:
```toml
[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies]
tauri-plugin-single-instance = "2"
```

### 2. gRPC 依存関係の削除

**Cargo.toml** から削除:
```toml
# [dependencies] から削除
sysinfo = { version = "0.30.11" }
interprocess = { version = "2.0.0" }
tonic = { version = "0.11.0" }
prost = { version = "0.12.4" }
tokio-stream = { version = "0.1.15", features = ["time"] }
stream-cancel = "0.8.2"

# [build-dependencies] から削除
tonic-build = { version = "0.11.0" }
```

**残す依存関係**:
- `tokio` - Mutex, spawn, oneshot で使用
- `futures` - 確認必要だが念のため残す

### 3. build.rs の簡略化

```rust
// Before
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tauri_build::build();
    tonic_build::compile_protos("proto/add_tab.proto")?;
    tonic_build::compile_protos("proto/new_window.proto")?;
    Ok(())
}

// After
fn main() {
    tauri_build::build();
}
```

### 4. lib.rs の更新

```rust
// Before
pub mod add_tab {
    tonic::include_proto!("add_tab");
}

pub mod new_window {
    tonic::include_proto!("new_window");
}

pub mod app;
pub mod grpc;
pub mod service;
pub mod utils;

// After
pub mod app;
pub mod service;
pub mod utils;
```

### 5. app/mod.rs の更新

#### 削除する関数
- `get_running_count()` - プロセス数カウント関数

#### 削除するインポート
- `sysinfo::System`
- `crate::grpc::{add_tab, new_window, server}`

#### setup() の変更

```rust
// Before
.setup(|app| {
    // ...menu setup...
    
    if get_running_count() > 1 {
        // gRPC クライアントとして動作
        match app.cli().matches() {
            Ok(matches) => match &matches.args.get("filepath").map(|v| v.value.clone()) {
                Some(Value::String(val)) => {
                    tokio::spawn(add_tab::transfer(val.to_string(), app.app_handle().clone()));
                }
                _ => {
                    tokio::spawn(new_window::open(app.app_handle().clone()));
                }
            },
            Err(_) => { app.handle().exit(0); }
        }
    } else {
        // gRPC サーバーを起動
        tokio::spawn(server::run_server(app.app_handle().clone()));
        // ... ウィンドウ復元処理 ...
    }
    Ok(())
})

// After
.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
    // 2番目のインスタンスが起動しようとした時のコールバック
    // args[0] = 実行ファイルパス, args[1..] = 引数
    let app = app.clone();
    
    tokio::spawn(async move {
        let state = app.state::<AppState>();
        
        // ファイルパス引数がある場合はタブ追加
        if args.len() > 1 {
            let filepath = args[1].clone();
            let label = state.active.lock().await.label.clone();
            if let Ok(_) = add_viewer_tab_state(&filepath, &label, &state).await {
                let windows = state.viewers.lock().await;
                if let Some(window_state) = windows.iter().find(|v| v.label == label) {
                    let _ = app.emit_to(&label, "viewer-state-changed", window_state);
                }
            }
            // アクティブウィンドウをフォーカス
            if let Some(window) = app.get_webview_window(&label) {
                let _ = window.set_focus();
            }
        } else {
            // 引数なしの場合は新規ウィンドウ
            if let Ok(label) = add_viewer_state(&state).await {
                let _ = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html".into()))
                    .title("Simple Image Viewer")
                    .maximized(true)
                    .build();
            }
        }
    });
}))
.setup(|app| {
    // ...menu setup...
    
    // ウィンドウ復元処理（gRPC サーバー起動は不要に）
    saved_state.viewers.into_iter().for_each(|v| {
        // ... 既存のウィンドウ復元ロジック ...
    });
    Ok(())
})
```

### 6. ファイル/ディレクトリの削除

```bash
# proto ディレクトリ
rm -rf src-tauri/proto/

# grpc モジュール
rm -rf src-tauri/src/grpc/
```

## 比較表

| 項目 | Before (gRPC) | After (single-instance) |
|------|---------------|------------------------|
| コード行数 | ~200行 | ~30行 |
| 依存クレート | 6+ | 1 |
| ビルド設定 | Proto コンパイル | なし |
| ポート管理 | ハードコード | OS ネイティブ |
| 信頼性 | カスタム実装 | Tauri 公式 |
| メンテナンス | 自己管理 | Tauri チーム |

## 注意事項

1. **CLI 引数の取得方法の変更**
   - Before: `app.cli().matches()` で filepath 引数を取得
   - After: `args` 配列から直接取得（args[0] は実行ファイル、args[1] 以降が引数）

2. **ウィンドウフォーカス**
   - `AppState.active` を使用してアクティブウィンドウを特定
   - `set_focus()` でフォーカスを移動

3. **tokio の維持**
   - `tokio::sync::Mutex` と `tokio::spawn` は引き続き使用
   - features は `["full"]` のまま維持
