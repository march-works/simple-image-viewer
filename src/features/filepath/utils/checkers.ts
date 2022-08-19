import { CompressedExtensions } from '../consts/compressed';
import { ImageExtensions } from '../consts/images';

export const isCompressedFile = (filepath: string): boolean => {
  return CompressedExtensions.some((ext) => filepath.endsWith(`.${ext}`));
};

export const isImageFile = (filepath: string): boolean => {
  return ImageExtensions.some((ext) => filepath.endsWith(`.${ext}`));
};
