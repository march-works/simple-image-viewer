import { Component, createSignal } from 'solid-js';
import { Thumbnail } from '../types/Thumbnail';
import fallback from '../../../assets/noimage.png';
import { FaSolidCheck } from 'solid-icons/fa';

type Props = {
  thumb: Thumbnail;
  onMarkedAsRead: (path: string) => void;
  onClick: (thumb: Thumbnail) => void;
};

export const Folder: Component<Props> = (props) => {
  const [isRead, setIsRead] = createSignal<boolean>(false);
  return (
    <div class="flex flex-col w-96 h-96 overflow-hidden relative">
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
      <img
        class="block cursor-pointer w-80 h-80 object-contain"
        onClick={() => props.onClick(props.thumb)}
        src={`data:image/jpeg;base64,${props.thumb.thumbnail}`}
        onError={(e) => (e.currentTarget.src = fallback)}
      />
      <div class="whitespace-nowrap text-ellipsis">{props.thumb.filename}</div>
    </div>
  );
};
