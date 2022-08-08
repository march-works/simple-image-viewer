import { FC } from "react";
import { DirectoryTree, File } from "../types/DirectoryTree";

type Props = {
  entries: DirectoryTree[];
  onClick: (entry: File) => void;
};

export const FileTree: FC<Props> = ({ entries, onClick }) => {
  return (
    <div className="pl-2">
      {entries.map((entry) =>
        entry.type === "File" ? (
          <div key={entry.path}>
            <button
              className="w-full text-neutral-100"
              onClick={() => onClick(entry)}
            >
              {entry.name}
            </button>
          </div>
        ) : (
          <FileTree entries={entry.children ?? []} onClick={onClick} />
        )
      )}
    </div>
  );
};
