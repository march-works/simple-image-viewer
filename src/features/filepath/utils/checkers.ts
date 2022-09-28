import { CompressedExtensions } from '../consts/Compressed';
import { ImageExtensions } from '../consts/Images';

export const isCompressedFile = (filepath: string): boolean => {
  return CompressedExtensions.some((ext) => filepath.endsWith(`.${ext}`));
};

export const isImageFile = (filepath: string): boolean => {
  return ImageExtensions.some((ext) => filepath.endsWith(`.${ext}`));
};
