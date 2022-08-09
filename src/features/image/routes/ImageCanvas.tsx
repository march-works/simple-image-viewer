import { readBinaryFile } from '@tauri-apps/api/fs';
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
      <button className="text-neutral-100" onClick={moveBackward}>
        {'<'}
      </button>
      <div className="flex content-center flex-1">
        <img
          src={`data:image/jpeg;base64,${data}`}
          alt={'please specify directory'}
          className="object-contain flex-1"
        />
      </div>
      <button className="text-neutral-100" onClick={moveForward}>
        {'>'}
      </button>
    </div>
  );
};
