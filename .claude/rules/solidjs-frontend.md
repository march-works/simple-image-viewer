# SolidJS Frontend Coding Rules

このドキュメントは SolidJS/TypeScript フロントエンドのコーディング規約を定義します。

## SolidJS プリミティブ

### createSignal

```typescript
// 基本的な状態管理
const [count, setCount] = createSignal(0);
const [viewing, setViewing] = createSignal<File | undefined>(undefined);

// 更新
setCount(10);
setCount((prev) => prev + 1);

// 読み取り（関数呼び出し）
console.log(count());
```

### createMemo

```typescript
// 派生状態（依存が変わったときのみ再計算）
const doubled = createMemo(() => count() * 2);

// 等価性チェックをカスタマイズ
import { equal } from 'fast-deep-equal';
const tree = createMemo(() => props.tree, undefined, { equals: equal });
```

### createEffect

```typescript
// 副作用の実行
createEffect(() => {
  const value = someSignal();
  console.log('Signal changed:', value);
});

// クリーンアップ付き
createEffect((prevCleanup) => {
  prevCleanup?.(); // 前回のクリーンアップ

  const subscription = subscribe(someSignal());
  return () => subscription.unsubscribe();
});
```

### onCleanup

```typescript
// コンポーネントのクリーンアップ
const MyComponent = () => {
    let unlistenRef: (() => void) | undefined;

    onMount(async () => {
        unlistenRef = await appWindow.listen('event', handler);
    });

    onCleanup(() => {
        unlistenRef?.();
    });

    return <div>...</div>;
};
```

## JSX 規約

### 属性名

```tsx
// ✅ class を使用
<div class="container">

// ❌ className は使用しない
<div className="container">

// イベントハンドラはキャメルケース
<button onClick={handleClick}>
<input onInput={handleInput}>
```

### 条件付きレンダリング

```tsx
// <Show> コンポーネント
<Show when={isLoading()} fallback={<Loading />}>
    <Content />
</Show>

// when の値をコールバックで受け取る
<Show when={user()}>
    {(user) => <UserProfile user={user()} />}
</Show>
```

### リストレンダリング

```tsx
// <For> コンポーネント
<For each={items()}>
    {(item, index) => (
        <div class="item" data-index={index()}>
            {item.name}
        </div>
    )}
</For>

// <Index> - インデックスが固定、値が変化する場合
<Index each={items()}>
    {(item, index) => <div>{item().name}</div>}
</Index>
```

### パターンマッチング

```tsx
// Switch/Match コンポーネント
<Switch fallback={<DefaultView />}>
  <Match when={status() === 'loading'}>
    <Loading />
  </Match>
  <Match when={status() === 'error'}>
    <Error />
  </Match>
  <Match when={status() === 'success'}>
    <Success />
  </Match>
</Switch>
```

## ts-pattern によるパターンマッチング

```typescript
import { match, P } from 'ts-pattern';

// 判別共用体のマッチング
const renderNode = (node: FileTree) =>
    match(node)
        .with({ Directory: P.any }, (nd) => (
            <DirectoryNode directory={nd.Directory} />
        ))
        .with({ File: { file_type: 'Image' } }, (nd) => (
            <ImageNode file={nd.File} />
        ))
        .with({ File: { file_type: 'Video' } }, (nd) => (
            <VideoNode file={nd.File} />
        ))
        .with({ File: { file_type: 'Zip' } }, (nd) => (
            <ZipNode file={nd.File} />
        ))
        .exhaustive();
```

## Tauri IPC 通信

### invoke

```typescript
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

// コマンド呼び出し
const result = await invoke<ResultType>('command_name', {
  label: appWindow.label,
  path: filePath,
});
```

### イベントリスナー

```typescript
const MyComponent = () => {
    let unlistenRef: (() => void) | undefined;

    onMount(async () => {
        const appWindow = getCurrentWindow();

        // イベント購読
        unlistenRef = await appWindow.listen<PayloadType>(
            'event-name',
            (event) => {
                setData(event.payload);
            }
        );
    });

    // 必ずクリーンアップ
    onCleanup(() => {
        unlistenRef?.();
    });

    return <div>...</div>;
};
```

## 型定義

### Props 型

```typescript
interface MyComponentProps {
    value: string;
    onChange?: (value: string) => void;
    children?: JSX.Element;
}

const MyComponent = (props: MyComponentProps) => {
    // props はリアクティブ、分割代入しない
    return <div>{props.value}</div>;
};
```

### IPC ペイロード型

```typescript
// Rust の構造体に対応
interface ViewerState {
  label: string;
  tabs: TabState[];
  activeTabIndex: number; // camelCase
}

interface TabState {
  key: string;
  viewing?: FileInfo;
  fileList: FileInfo[];
}
```

## スタイリング (TailwindCSS)

```tsx
// 動的クラス
<div class={`base-class ${isActive() ? 'active' : ''}`}>

// classList (SolidJS 固有)
<div classList={{
    'base-class': true,
    'active': isActive(),
    'disabled': isDisabled(),
}}>
```

## パフォーマンス最適化

### バッチ更新

```typescript
import { batch } from 'solid-js';

// 複数の状態を一度に更新
batch(() => {
  setCount(10);
  setName('New Name');
  setItems([]);
});
```

### untrack

```typescript
import { untrack } from 'solid-js';

createEffect(() => {
  const trackedValue = trackedSignal();

  // この読み取りは依存関係に含まれない
  const untrackedValue = untrack(() => otherSignal());
});
```

## コンポーネント構造

```typescript
import { createSignal, onCleanup, onMount, Show, For } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface Props {
    initialValue: string;
}

const MyComponent = (props: Props) => {
    // 1. シグナル定義
    const [value, setValue] = createSignal(props.initialValue);
    const [loading, setLoading] = createSignal(false);

    // 2. リスナー参照
    let unlistenRef: (() => void) | undefined;

    // 3. ライフサイクル
    onMount(async () => {
        const appWindow = getCurrentWindow();
        unlistenRef = await appWindow.listen('event', handler);
    });

    onCleanup(() => {
        unlistenRef?.();
    });

    // 4. ハンドラ
    const handleClick = async () => {
        setLoading(true);
        try {
            await invoke('command');
        } finally {
            setLoading(false);
        }
    };

    // 5. レンダリング
    return (
        <div class="container">
            <Show when={!loading()} fallback={<Loading />}>
                <button onClick={handleClick}>{value()}</button>
            </Show>
        </div>
    );
};

export default MyComponent;
```

## 禁止事項

```typescript
// ❌ props を分割代入しない（リアクティビティが失われる）
const MyComponent = ({ value, onChange }: Props) => {
    return <div>{value}</div>;  // リアクティブではない
};

// ✅ props はそのまま使用
const MyComponent = (props: Props) => {
    return <div>{props.value}</div>;  // リアクティブ
};

// ❌ シグナルを直接参照しない
const value = signal;  // 関数オブジェクト

// ✅ 関数として呼び出す
const value = signal();  // 値を取得
```
