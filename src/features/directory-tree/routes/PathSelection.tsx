import { open } from "@tauri-apps/api/dialog";
import { FileEntry, readDir } from "@tauri-apps/api/fs";
import { FC } from "react";
import { FileTree } from "../components/FileTree";
import { DirectoryTree } from "../types/DirectoryTree";

type Props = {
  tree: DirectoryTree[];
  setTree: (tree: DirectoryTree[]) => void;
  onSelectedChanged: (entries: string) => void;
};

export const PathSelection: FC<Props> = ({
  tree,
  setTree,
  onSelectedChanged,
}) => {
  const convertEntryToTree = (entry: FileEntry): DirectoryTree => {
    if (entry.children === null || entry.children === undefined) {
      return {
        type: "File",
        name: entry.name ?? "",
        path: entry.path,
      };
    }
    return {
      type: "Directory",
      name: entry.name ?? "",
      path: entry.path,
      children: entry.children.map(convertEntryToTree),
    };
  };

  const handleOnClick = async () => {
    const dir = await open({
      directory: true,
    });
    if (Array.isArray(dir)) {
      return;
    }
    if (!dir) {
      return;
    }
    const entries = await readDir(dir, {
      recursive: true,
    });
    setTree(entries.map(convertEntryToTree));
  };

  return (
    <div className="flex flex-col space-y-2">
      <button onClick={handleOnClick} className="bg-neutral-200 rounded">
        Open Directory
      </button>
      <div className="overflow-y-auto">
        <FileTree
          entries={tree}
          onClick={(entry) => onSelectedChanged(entry.path)}
        />
      </div>
    </div>
  );
};
