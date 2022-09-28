export const getFileNameWithoutExtension = (filepath: string) => {
  return filepath.split('\\').pop()?.split('/').pop()?.split('.')[0] ?? '';
};

export const getParentDirectoryName = (filepath: string) => {
  return (
    filepath.replaceAll(/\\/gi, '/').split('/').slice(undefined, -1).pop() ?? ''
  );
};

export const getParentDirectoryPath = (filepath: string) => {
  const bsIdx = filepath.lastIndexOf('\\');
  const splited = filepath.slice(0, bsIdx >= 0 ? bsIdx : undefined);
  const slIndx = splited.lastIndexOf('/');
  return splited.slice(0, slIndx >= 0 ? slIndx : undefined);
};
