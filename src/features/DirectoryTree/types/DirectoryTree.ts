export type Directory = {
  type: 'Directory';
  name: string;
  path: string;
  children: DirectoryTree[];
};

export type Image = {
  type: 'Image';
  name: string;
  path: string;
};

export type Video = {
  type: 'Video';
  name: string;
  path: string;
};

export type Zip = {
  type: 'Zip';
  name: string;
  path: string;
};

export type File = Image | Video | Zip;

export type DirectoryTree = Directory | File;
