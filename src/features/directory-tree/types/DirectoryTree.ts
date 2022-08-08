export type Directory = {
  type: "Directory";
  name: string;
  path: string;
  children: DirectoryTree[];
};

export type File = {
  type: "File";
  name: string;
  path: string;
};

export type DirectoryTree = Directory | File;
