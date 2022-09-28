import { invoke } from '@tauri-apps/api/tauri';
import { Component, createEffect, createSignal } from 'solid-js';
import { match } from 'ts-pattern';
import { File, Zip } from '../../DirectoryTree/types/DirectoryTree';

type Props = {
  viewing?: File | Zip;
  moveForward: () => void;
  moveBackward: () => void;
};

export const ImageCanvas: Component<Props> = (props) => {
  const [data, setData] = createSignal<string>('');

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
      <div class="relative flex flex-1 content-center justify-center">
        <img class="object-contain" src={`data:image/jpeg;base64,${data()}`} />
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
