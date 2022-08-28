import { LeftOutlined, RightOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/tauri';
import { FC, useCallback, useEffect, useState } from 'react';
import { match } from 'ts-pattern';
import { File, Zip } from '../../directory-tree/types/DirectoryTree';

type Props = {
  viewing?: File | Zip;
  moveForward: () => void;
  moveBackward: () => void;
};

export const ImageCanvas: FC<Props> = ({
  viewing,
  moveForward,
  moveBackward,
}) => {
  const [data, setData] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(true);

  const convertPathToData = useCallback(async (file: File) => {
    if (file.path === '') return '';
    return invoke<string>('open_file_image', { filepath: file.path });
  }, []);

  const readImageInZip = useCallback(async (file: Zip) => {
    return invoke<string>('read_image_in_zip', {
      path: file.path,
      filename: file.name,
    });
  }, []);

  useEffect(() => {
    setLoading(true);
    match(viewing)
      .with({ type: 'File' }, (file) =>
        convertPathToData(file).then((converted) => {
          setData(converted);
          setLoading(false);
        })
      )
      .with({ type: 'Zip' }, (file) =>
        readImageInZip(file).then((binary) => {
          setData(binary);
          setLoading(false);
        })
      )
      .with(undefined, () => {
        // do nothing
      })
      .exhaustive();
  }, [convertPathToData, viewing]);

  return (
    <div className="flex flex-row content-center" style={{ flex: 4 }}>
      <div className="flex items-center cursor-pointer" onClick={moveBackward}>
        <LeftOutlined className="p-2 text-xl" />
      </div>
      <div className="flex content-center justify-center flex-1">
        {loading ? (
          <div className="self-center object-fit">Loading</div>
        ) : (
          <img
            className="object-contain"
            src={`data:image/jpeg;base64,${data}`}
          />
        )}
      </div>
      <div className="flex items-center cursor-pointer" onClick={moveForward}>
        <RightOutlined className="p-2 text-xl" />
      </div>
    </div>
  );
};
