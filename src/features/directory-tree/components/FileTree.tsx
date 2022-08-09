import { FC } from 'react';
import { DirectoryTree, File } from '../types/DirectoryTree';

type Props = {
  selected: string;
  entries: DirectoryTree[];
  onClick: (entry: File) => void;
};

export const FileTree: FC<Props> = ({ selected, entries, onClick }) => {
  return (
    <div className="ml-5">
      {entries.map((entry) => {
        const bgColor =
          entry.path === selected ? 'bg-neutral-200' : 'bg-neutral-900';
        const txtColor =
          entry.path === selected ? 'text-neutral-900' : 'text-neutral-100';
        return entry.type === 'File' ? (
          <div key={entry.path}>
            <button
              className={`w-full text-left ${bgColor} ${txtColor}`}
              onClick={() => onClick(entry)}
            >
              {entry.name}
            </button>
          </div>
        ) : (
          <FileTree
            key={entry.path}
            selected={selected}
            entries={entry.children ?? []}
            onClick={onClick}
          />
        );
      })}
    </div>
  );
};
