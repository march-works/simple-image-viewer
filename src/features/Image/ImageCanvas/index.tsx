import { convertFileSrc, invoke } from '@tauri-apps/api/tauri';
import {
  Component,
  createEffect,
  createResource,
  createSignal,
  Match,
  on,
  onCleanup,
  onMount,
  Show,
  Switch,
} from 'solid-js';
import { match } from 'ts-pattern';
import {
  Image,
  Zip,
  File,
  Video,
} from '../../DirectoryTree/types/DirectoryTree';
import {
  HiSolidChevronLeft,
  HiSolidChevronRight,
  HiSolidZoomIn,
  HiSolidZoomOut,
} from 'solid-icons/hi';
import 'video.js/dist/video-js.css';
import '@videojs/themes/dist/fantasy/index.css';

type Props = {
  viewing?: File;
  moveForward: () => void;
  moveBackward: () => void;
};

export const ImageCanvas: Component<Props> = (props) => {
  // const [data, setData] = createSignal<Pick<File, 'type'> & { data: string }>();
  const [isDragging, setIsDragging] = createSignal(false);
  const [initialPosition, setInitialPosition] = createSignal({ x: 0, y: 0 });
  const [imageScale, setImageScale] = createSignal<number>(1);
  const [position, setPosition] = createSignal({ x: 0, y: 0 });

  const handleWheel = (e: WheelEvent) => {
    e.preventDefault();
    setImageScale((prev) =>
      Math.min(Math.max(0.1, prev + (e.deltaY > 0 ? -0.1 : 0.1)), 3)
    );
  };

  const zoomIn = () => {
    setImageScale((prev) => Math.min(Math.max(0.1, prev + 0.1), 3));
  };

  const zoomOut = () => {
    setImageScale((prev) => Math.min(Math.max(0.1, prev - 0.1), 3));
  };

  const handleOnKeyDown = (event: KeyboardEvent) => {
    event.preventDefault();
    if (event.ctrlKey && event.key === 'i') zoomIn();
    else if (event.ctrlKey && event.key === 'o') zoomOut();
  };

  onMount(() => {
    document.addEventListener('keydown', handleOnKeyDown, false);
  });

  onCleanup(() => {
    document.removeEventListener('keydown', handleOnKeyDown, false);
  });

  const handlePositionChange = (newPosition: { x: number; y: number }) => {
    setPosition(newPosition);
  };

  const resetStatus = () => {
    setImageScale(1);
    setPosition({ x: 0, y: 0 });
  };

  const convertToLocalPath = async (file: Image | Video) => {
    return convertFileSrc(file.path);
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
      handlePositionChange({
        x: position().x + dx,
        y: position().y + dy,
      });
      setInitialPosition({ x: event.clientX, y: event.clientY });
    }
  };

  const [data] = createResource(
    () => ({ ...props }),
    () =>
      match(props.viewing)
        .with({ type: 'Image' }, (file) => convertToLocalPath(file))
        .with({ type: 'Video' }, (file) => convertToLocalPath(file))
        .with({ type: 'Zip' }, (file) => readImageInZip(file))
        .otherwise(() => undefined)
  );

  createEffect(
    on(
      () => props.viewing,
      () => resetStatus(),
      { defer: true }
    )
  );

  return (
    <div class="flex flex-row content-center" style={{ flex: 4 }}>
      <div
        class="flex cursor-pointer items-center opacity-50 transition-colors hover:bg-neutral-800 hover:opacity-100"
        onClick={() => props.moveBackward()}
      >
        <HiSolidChevronLeft class="text-6xl" />
      </div>
      <div class="relative flex content-center" style={{ flex: 2 }}>
        <div
          class="max-w-full max-h-full object-cover object-center relative flex flex-1 content-center justify-center overflow-hidden"
          onMouseDown={handleMouseDown}
          onMouseUp={handleMouseUp}
          onMouseMove={handleMouseMove}
          onWheel={handleWheel}
        >
          <Switch>
            <Match when={props.viewing?.type === 'Image'}>
              <img
                class="w-full h-full object-contain"
                src={data()}
                style={{
                  transform: `scale(${imageScale()}) translate(${
                    position().x
                  }px, ${position().y}px)`,
                  position: 'absolute',
                  left: '0',
                  top: '0',
                }}
              />
            </Match>
            <Match when={props.viewing?.type === 'Video'}>
              <video
                class="video-js vjs-theme-fantasy w-full h-full object-contain"
                controls
                preload="auto"
                src={data()}
              />
            </Match>
            <Match when={props.viewing?.type === 'Zip'}>
              <img
                class="w-full h-full object-contain"
                src={`data:image/jpeg;base64,${data()}`}
                style={{
                  transform: `scale(${imageScale()}) translate(${
                    position().x
                  }px, ${position().y}px)`,
                  position: 'absolute',
                  left: '0',
                  top: '0',
                }}
              />
            </Match>
          </Switch>
        </div>
        <Show
          when={
            props.viewing?.type === 'Image' || props.viewing?.type === 'Zip'
          }
        >
          <div class="fixed bottom-3 left-0 w-full flex justify-center gap-10">
            <div
              class="flex cursor-pointer opacity-20 transition-colors hover:opacity-100 items-center justify-center"
              onClick={zoomIn}
            >
              <HiSolidZoomIn class="text-3xl" />
            </div>
            <div
              class="flex cursor-pointer opacity-20 transition-colors hover:opacity-100 items-center justify-center"
              onClick={zoomOut}
            >
              <HiSolidZoomOut class="text-3xl" />
            </div>
          </div>
        </Show>
      </div>
      <div
        class="flex cursor-pointer items-center opacity-50 transition-colors hover:bg-neutral-800 hover:opacity-100"
        onClick={() => props.moveForward()}
      >
        <HiSolidChevronRight class="text-6xl" />
      </div>
    </div>
  );
};
