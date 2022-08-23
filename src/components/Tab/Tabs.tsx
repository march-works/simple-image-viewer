import { CloseOutlined, FolderOpenOutlined } from '@ant-design/icons';
import { FC, ReactElement } from 'react';

type TabInfo = {
  key: string;
  title: string;
};

type Props = {
  viewing?: string;
  tabs: TabInfo[];
  handleOnClick: (key: string) => void;
  handleOnClose: (key: string) => void;
  handleOnAdd: () => void;
  children?: ReactElement | ReactElement[];
};

export const Tabs: FC<Props> = ({
  viewing,
  tabs,
  handleOnClick,
  handleOnClose,
  handleOnAdd,
  children,
}) => {
  return (
    <div className="flex flex-col flex-1 w-full relative">
      <div className="flex flex-row flex-none h-8 bg-neutral-800 w-full">
        {tabs.map((tab) => (
          <div
            key={tab.key}
            className={
              'flex flex-row w-48 justify-between p-1 m-t-2 border-2 border-b-0 rounded-t-md border-neutral-500 min-w-0' +
              (tab.key === viewing ? ' bg-neutral-500' : '')
            }
            onMouseDown={(e) => e.button === 1 && handleOnClose(tab.key)}
          >
            <div
              className="flex-1 self-center cursor-pointer truncate"
              onClick={() => handleOnClick(tab.key)}
            >
              {tab.title}
            </div>
            <div
              className="flex flex-col justify-center cursor-pointer w-4"
              onClick={() => handleOnClose(tab.key)}
            >
              <CloseOutlined />
            </div>
          </div>
        ))}
        <div className="flex flex-col shrink-0 w-8 h-8 border-2 rounded-full ml-1 items-center justify-center border-neutral-500">
          <FolderOpenOutlined className="text-xl" onClick={handleOnAdd} />
        </div>
      </div>
      <div className="relative" style={{ height: 'calc(100% - 2rem)' }}>
        {Array.isArray(children)
          ? children.map((child) => (
              <div
                key={child.key}
                className={`w-full h-full${
                  child.key === viewing ? '' : ' hidden'
                }`}
              >
                {child}
              </div>
            ))
          : children}
      </div>
    </div>
  );
};
