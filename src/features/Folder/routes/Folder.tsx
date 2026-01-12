import { Show, createResource, createSignal } from 'solid-js';
import type { Component } from 'solid-js';
import type { Thumbnail } from '../types/Thumbnail';
import fallback from '../../../assets/noimage.png';
import { FaSolidCheck } from 'solid-icons/fa';
import { convertFileSrc } from '@tauri-apps/api/core';

type Props = {
  thumb: Thumbnail;
  showMarkAsRead: boolean;
  isHighlighted: boolean;
  onMarkedAsRead: (path: string) => void;
  onClick: (thumb: Thumbnail) => void;
};

export const Folder: Component<Props> = (props) => {
  const [isRead, setIsRead] = createSignal<boolean>(false);
  const [data] = createResource(
    () => props.thumb.thumbpath,
    () =>
      props.thumb.thumbpath ? convertFileSrc(props.thumb.thumbpath) : fallback,
  );
  return (
    <div
      class={`flex flex-col w-48 h-48 overflow-hidden relative rounded-lg transition-all ${
        props.isHighlighted
          ? 'bg-blue-900/40 ring-2 ring-blue-500'
          : 'bg-transparent'
      }`}
    >
      <Show when={props.showMarkAsRead}>
        <button
          class={`absolute top-0 right-4 ${
            isRead() ? 'text-green-400' : 'text-white'
          }`}
          onClick={() => {
            setIsRead(!isRead());
            props.onMarkedAsRead(props.thumb.path);
          }}
        >
          <FaSolidCheck class="w-6 h-6" />
        </button>
      </Show>
      <img
        class="block cursor-pointer w-40 h-40 object-contain"
        onClick={() => props.onClick(props.thumb)}
        src={data()}
        loading="lazy"
        onError={(e) => (e.currentTarget.src = fallback)}
      />
      <div class="whitespace-nowrap text-ellipsis">{props.thumb.filename}</div>
    </div>
  );
};
