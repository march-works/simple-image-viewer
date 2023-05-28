import { convertFileSrc, invoke } from '@tauri-apps/api/tauri';
import {
  Component,
  createEffect,
  createSignal,
  Match,
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
  zoomIn: () => void;
  zoomOut: () => void;
  imageScale: number;
  position: { x: number; y: number };
  onPositionChange: (position: { x: number; y: number }) => void;
  handleWheel: (e: WheelEvent) => void;
};

export const ImageCanvas: Component<Props> = (props) => {
  const [data, setData] = createSignal<Pick<File, 'type'> & { data: string }>();
  const [isDragging, setIsDragging] = createSignal(false);
  const [initialPosition, setInitialPosition] = createSignal({ x: 0, y: 0 });

  const convertPathToData = async (file: Image) => {
    if (file.path === '') return '';
    return invoke<string>('open_file_image', { filepath: file.path });
  };

  const convertToLocalPath = async (file: Video) => {
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
      props.onPositionChange({
        x: props.position.x + dx,
        y: props.position.y + dy,
      });
      setInitialPosition({ x: event.clientX, y: event.clientY });
    }
  };

  createEffect(() => {
    match(props.viewing)
      .with({ type: 'Image' }, (file) =>
        convertPathToData(file).then((converted) => {
          setData({
            type: 'Image',
            data: converted,
          });
        })
      )
      .with({ type: 'Video' }, (file) =>
        convertToLocalPath(file).then((converted) => {
          setData({
            type: 'Video',
            data: converted,
          });
        })
      )
      .with({ type: 'Zip' }, (file) =>
        readImageInZip(file).then((binary) => {
          setData({
            type: 'Zip',
            data: binary,
          });
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
        <HiSolidChevronLeft class="text-6xl" />
      </div>
      <div class="relative flex content-center" style={{ flex: 2 }}>
        <div
          class="max-w-full max-h-full object-cover object-center relative flex flex-1 content-center justify-center overflow-hidden"
          onMouseDown={handleMouseDown}
          onMouseUp={handleMouseUp}
          onMouseMove={handleMouseMove}
          onWheel={(e) => props.handleWheel(e)}
        >
          <Switch>
            <Match when={data()?.type === 'Image' || data()?.type === 'Zip'}>
              <img
                class="w-full h-full object-contain"
                src={`data:image/jpeg;base64,${data()?.data}`}
                style={{
                  transform: `scale(${props.imageScale}) translate(${props.position.x}px, ${props.position.y}px)`,
                  position: 'absolute',
                  left: '0',
                  top: '0',
                }}
              />
            </Match>
            <Match when={data()?.type === 'Video'}>
              <video
                class="video-js vjs-theme-fantasy w-full h-full object-contain"
                controls
                preload="auto"
                src={data()?.data}
              />
            </Match>
          </Switch>
        </div>
        <Show when={data()?.type === 'Image' || data()?.type === 'Zip'}>
          <div class="fixed bottom-3 left-0 w-full flex justify-center gap-10">
            <div
              class="flex cursor-pointer opacity-20 transition-colors hover:opacity-100 items-center justify-center"
              onClick={() => props.zoomIn()}
            >
              <HiSolidZoomIn class="text-3xl" />
            </div>
            <div
              class="flex cursor-pointer opacity-20 transition-colors hover:opacity-100 items-center justify-center"
              onClick={() => props.zoomOut()}
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
