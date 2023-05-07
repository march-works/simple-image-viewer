import { invoke } from '@tauri-apps/api/tauri';
import { Component, createEffect, createSignal } from 'solid-js';
import { match } from 'ts-pattern';
import { File, Zip } from '../../DirectoryTree/types/DirectoryTree';

type Props = {
  viewing?: File | Zip;
  moveForward: () => void;
  moveBackward: () => void;
  zoomIn: () => void;
  zoomOut: () => void;
  imageScale: number;
  position: { x: number; y: number };
  onPositionChange: (position: { x: number; y: number }) => void;
  handleWheel: (e: WheelEvent) => void;
};

export const ImageCanvas: Component<Props> = (props) => {
  const [data, setData] = createSignal<string>('');
  const [isDragging, setIsDragging] = createSignal(false);
  const [initialPosition, setInitialPosition] = createSignal({ x: 0, y: 0 });

  const convertPathToData = async (file: File) => {
    if (file.path === '') return '';
    return invoke<string>('open_file_image', { filepath: file.path });
  };

  const readImageInZip = async (file: Zip) => {
    return invoke<string>('read_image_in_zip', {
      path: file.path,
      filename: file.name,
    });
  };
  const handleMouseDown = (event: MouseEvent) => {
    event.preventDefault();
    if (event.button === 0) {
      setIsDragging(true);
      setInitialPosition({ x: event.clientX, y: event.clientY });
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  const handleMouseMove = (event: MouseEvent) => {
    if (isDragging()) {
      const dx = event.clientX - initialPosition().x;
      const dy = event.clientY - initialPosition().y;
      props.onPositionChange({
        x: props.position.x + dx,
        y: props.position.y + dy,
      });
      setInitialPosition({ x: event.clientX, y: event.clientY });
    }
  };

  createEffect(() => {
    match(props.viewing)
      .with({ type: 'File' }, (file) =>
        convertPathToData(file).then((converted) => {
          setData(converted);
        })
      )
      .with({ type: 'Zip' }, (file) =>
        readImageInZip(file).then((binary) => {
          setData(binary);
        })
      )
      .with(undefined, () => {
        // do nothing
      })
      .exhaustive();
  });

  return (
    <div class="flex flex-row content-center" style={{ flex: 4 }}>
      <div
        class="flex cursor-pointer items-center opacity-50 transition-colors hover:bg-neutral-800 hover:opacity-100"
        onClick={() => props.moveBackward()}
      >
        <i class="fa-solid fa-chevron-left p-2 text-4xl" />
      </div>
      <div class="relative flex content-center" style={{ flex: 2 }}>
        <div
          class="max-w-full max-h-full object-cover object-center relative flex flex-1 content-center justify-center overflow-hidden"
          onMouseDown={handleMouseDown}
          onMouseUp={handleMouseUp}
          onMouseMove={handleMouseMove}
          onWheel={props.handleWheel}
        >
          <img
            class="absolute inset-0 m-auto object-contain"
            src={`data:image/jpeg;base64,${data()}`}
            style={{
              transform: `scale(${props.imageScale}) translate(${props.position.x}px, ${props.position.y}px)`,
              position: 'absolute',
              left: '0',
              top: '0',
            }}
          />
        </div>
        <div class="absolute fixed bottom-0 left-0 w-full flex justify-center items-end">
          <button
            class="w-20 h-20 flex cursor-pointer opacity-50 transition-colors hover:opacity-100 items-center justify-center"
            onClick={() => props.zoomIn()}
          >
            <i class="fa-solid fa-regular fa-magnifying-glass-plus p-2 text-4xl" />
          </button>
          <button
            class="w-20 h-20 flex cursor-pointer opacity-50 transition-colors hover:opacity-100 items-center justify-center"
            onClick={() => props.zoomOut()}
          >
            <i class=" fa-regular fa-magnifying-glass-minus p-2 text-4xl" />
          </button>
        </div>
      </div>
      <div
        class="flex cursor-pointer items-center opacity-50 transition-colors hover:bg-neutral-800 hover:opacity-100"
        onClick={() => props.moveForward()}
      >
        <i class="fa-solid fa-chevron-right p-2 text-4xl" />
      </div>
    </div>
  );
};
