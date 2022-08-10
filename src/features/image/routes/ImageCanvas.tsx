import { LeftOutlined, RightOutlined } from '@ant-design/icons';
import { readBinaryFile } from '@tauri-apps/api/fs';
import SkeletonImage from 'antd/lib/skeleton/Image';
import { fromByteArray } from 'base64-js';
import { FC, useCallback, useEffect, useState } from 'react';

type Props = {
  path: string;
  moveForward: () => void;
  moveBackward: () => void;
};

export const ImageCanvas: FC<Props> = ({ path, moveForward, moveBackward }) => {
  const [data, setData] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(true);

  const convertPathToData = useCallback(async (path: string) => {
    if (path === '') return '';
    const ret = await readBinaryFile(path);
    return fromByteArray(ret);
  }, []);

  useEffect(() => {
    setLoading(true);
    convertPathToData(path).then((converted) => {
      setData(converted);
      setLoading(false);
    });
  }, [convertPathToData, path]);

  return (
    <div className="flex flex-row content-center" style={{ flex: 4 }}>
      <div className="flex items-center cursor-pointer" onClick={moveBackward}>
        <LeftOutlined className="p-2 text-xl" />
      </div>
      <div className="flex content-center justify-center flex-1">
        {loading ? (
          <SkeletonImage className="self-center object-fit" />
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
