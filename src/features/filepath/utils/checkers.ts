import { CompressedExtensions } from '../consts/compressed';
import { ImageExtensions } from '../consts/images';
import { VideoExtensions } from '../consts/videos';

export const isCompressedFile = (filepath: string): boolean => {
  return CompressedExtensions.some((ext) => filepath.endsWith(`.${ext}`));
};

export const isImageFile = (filepath: string): boolean => {
  return ImageExtensions.some((ext) => filepath.endsWith(`.${ext}`));
};

export const isVideoFile = (filepath: string): boolean => {
  return VideoExtensions.some((ext) => filepath.endsWith(`.${ext}`));
};
