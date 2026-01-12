import { createSignal, onMount, onCleanup, Show } from 'solid-js';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface DownloadProgress {
  filename: string;
  downloaded: number;
  total: number;
  percentage: number;
  status: 'starting' | 'downloading' | 'verifying' | 'completed' | 'failed';
}

export function ModelDownloadProgress() {
  const [progress, setProgress] = createSignal<DownloadProgress | null>(null);
  const [visible, setVisible] = createSignal(false);
  let unlisten: UnlistenFn | undefined;

  onMount(async () => {
    unlisten = await listen<DownloadProgress>(
      'model-download-progress',
      (event) => {
        setProgress(event.payload);

        // 完了またはエラー時は3秒後に非表示
        if (
          event.payload.status === 'completed' ||
          event.payload.status === 'failed'
        ) {
          setTimeout(() => setVisible(false), 3000);
        } else {
          setVisible(true);
        }
      },
    );
  });

  onCleanup(() => {
    unlisten?.();
  });

  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024)
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  };

  const getStatusText = (status: DownloadProgress['status']): string => {
    switch (status) {
      case 'starting':
        return '開始中...';
      case 'downloading':
        return 'ダウンロード中...';
      case 'verifying':
        return '検証中...';
      case 'completed':
        return '完了';
      case 'failed':
        return '失敗';
    }
  };

  return (
    <Show when={visible() && progress()}>
      <div class="fixed bottom-4 right-4 bg-gray-800 text-white p-4 rounded-lg shadow-lg min-w-80 z-50">
        <div class="text-sm font-medium mb-2">MLモデルのダウンロード</div>
        <div class="text-xs text-gray-300 mb-2">{progress()?.filename}</div>
        <div class="w-full bg-gray-700 rounded-full h-2 mb-2">
          <div
            class="bg-blue-500 h-2 rounded-full transition-all duration-300"
            style={{ width: `${progress()?.percentage ?? 0}%` }}
          />
        </div>
        <div class="flex justify-between text-xs text-gray-400">
          <span>{getStatusText(progress()?.status ?? 'starting')}</span>
          <span>
            {formatBytes(progress()?.downloaded ?? 0)} /{' '}
            {formatBytes(progress()?.total ?? 0)}
          </span>
        </div>
      </div>
    </Show>
  );
}
