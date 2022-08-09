import { LeftOutlined, RightOutlined } from '@ant-design/icons';
import { readBinaryFile } from '@tauri-apps/api/fs';
import { Image } from 'antd';
import { fromByteArray } from 'base64-js';
import { FC, useCallback, useEffect, useState } from 'react';

type Props = {
  path: string;
  moveForward: () => void;
  moveBackward: () => void;
};

export const ImageCanvas: FC<Props> = ({ path, moveForward, moveBackward }) => {
  const [data, setData] = useState<string>('');

  const convertPathToData = useCallback(async (path: string) => {
    if (path === '') return '';
    const ret = await readBinaryFile(path);
    return fromByteArray(ret);
  }, []);

  useEffect(() => {
    convertPathToData(path).then(setData);
  }, [convertPathToData, path]);

  return (
    <div className="flex content-center" style={{ flex: 4 }}>
      <div className='flex items-center cursor-pointer' onClick={moveBackward}>
        <LeftOutlined className='p-2 text-xl' />
      </div>
      <div className='flex content-center flex-1'>
        <Image
          rootClassName='flex content-center flex-1'
          className="object-contain flex-1"
          src={`data:image/jpeg;base64,${data}`}
          preview={false}
        />
      </div>
      <div className='flex items-center cursor-pointer' onClick={moveForward}>
        <RightOutlined className='p-2 text-xl' />
      </div>
    </div>
  );
};
