# Architecture Rules - Simple Image Viewer

このドキュメントはプロジェクトのアーキテクチャ原則を定義します。

## バックエンド 3 層アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                     app/ (Tauri Command Layer)               │
│  ・#[tauri::command] エントリーポイント                        │
│  ・AppHandle 操作、イベント emit                              │
│  ・service/ を呼び出して状態操作を委譲                         │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                  service/ (State Management Layer)           │
│  ・AppState の操作とビジネスロジック                          │
│  ・状態の読み書き、ロック管理                                 │
│  ・AppHandle や emit 処理を直接扱わない                       │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                    utils/ (Utility Layer)                    │
│  ・汎用的なヘルパー関数                                       │
│  ・ファイル操作、watcher 管理など                             │
│  ・状態操作、ウィンドウ操作を行わない                          │
└─────────────────────────────────────────────────────────────┘
```

### 各層の責務

| レイヤー   | ファイル                                                           | 責務                                                                |
| ---------- | ------------------------------------------------------------------ | ------------------------------------------------------------------- |
| `app/`     | `viewer.rs`, `explorer.rs`                                         | Tauri コマンド定義、`AppHandle` 経由のイベント emit、ウィンドウ操作 |
| `service/` | `app_state.rs`, `viewer_state.rs`, `explorer_state.rs`, `types.rs` | 状態定義、状態変更ロジック、Mutex/RwLock 管理                       |
| `utils/`   | `file_utils.rs`, `watcher_utils.rs`, `thumbnail_utils.rs`          | ファイル種別判定、パス操作、サムネイル生成                          |

### 禁止事項

#### app/ レイヤー

- ❌ `state.viewers.lock().await` で直接状態を変更する（service 経由で行う）
- ❌ 複雑なビジネスロジックを記述する

#### service/ レイヤー

- ❌ `AppHandle` を受け取る
- ❌ `app.emit()` や `app.emit_to()` を呼び出す
- ❌ ウィンドウ操作を行う

#### utils/ レイヤー

- ❌ `AppState` を参照する
- ❌ Tauri 固有の型を使用する

## フロントエンド構造

```
src/
├── pages/              # ルートレベルのページ
│   ├── viewer/         # Viewer ウィンドウ
│   │   ├── index.tsx   # エントリーポイント
│   │   ├── Viewer.tsx  # メインコンポーネント
│   │   └── ViewerTab.tsx
│   └── explorer/       # Explorer ウィンドウ
│       ├── index.tsx
│       ├── Explorer.tsx
│       └── ExplorerTab.tsx
├── features/           # 機能モジュール
│   ├── DirectoryTree/  # ディレクトリツリー表示
│   ├── Image/          # 画像表示・操作
│   ├── Explorer/       # Explorer 固有機能
│   └── Folder/         # フォルダ操作
└── components/         # 共通 UI コンポーネント
    ├── Loading/
    ├── Pagination/
    └── ModelDownloadProgress/
```

### フロントエンド原則

1. **pages/**: ルーティング対象、タブ管理、ウィンドウレベルの状態管理
2. **features/**: 機能単位でカプセル化、独自の `components/`, `routes/`, `types/` を持てる
3. **components/**: 汎用 UI、特定機能に依存しない

## マルチウィンドウ協調

### ウィンドウ識別

- Viewer: `viewer-0`, `viewer-1`, ...
- Explorer: `explorer-0`, `explorer-1`, ...

### 状態同期フロー

```
1. Frontend: invoke('command_name', { label, ...params })
       ↓
2. Backend: app/command → service/state mutation
       ↓
3. Backend: app.emit_to(&label, "event-name", payload)
       ↓
4. Frontend: appWindow.listen("event-name", handler)
       ↓
5. Frontend: setSignal(payload) → Component re-render
```

### クロスウィンドウ通知

- Viewer のアクティブディレクトリ変更 → 全 Explorer に通知
- Explorer からファイル選択 → 対応 Viewer に通知

## ファイル構成規則

### 新規 Tauri コマンド追加時

1. `app/` に `#[tauri::command]` 関数を追加
2. 状態操作が必要なら `service/` にヘルパー関数を追加
3. `lib.rs` の `invoke_handler` にコマンドを登録
4. フロントエンドで `invoke()` を呼び出し

### 新規機能追加時

1. `src/features/` に機能フォルダを作成
2. 必要に応じて `components/`, `routes/`, `types/` サブフォルダを作成
3. 共通 UI は `src/components/` に配置
