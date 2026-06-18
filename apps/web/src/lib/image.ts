const MAX_DIMENSION = 1440;
const JPEG_QUALITY = 0.82;
const MAX_BYTES = 1.5 * 1024 * 1024;

export async function prepareImageForUpload(file: File): Promise<string> {
  if (!file.type.startsWith('image/') && !/\.(jpe?g|png|gif|webp|heic|heif)$/i.test(file.name)) {
    throw new Error('Apenas imagens são suportadas');
  }

  const bitmap = await createImageBitmap(file);
  const longest = Math.max(bitmap.width, bitmap.height);
  const scale = longest > MAX_DIMENSION ? MAX_DIMENSION / longest : 1;
  const width = Math.round(bitmap.width * scale);
  const height = Math.round(bitmap.height * scale);

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');
  if (!ctx) {
    bitmap.close();
    throw new Error('Falha ao processar imagem');
  }
  ctx.drawImage(bitmap, 0, 0, width, height);
  bitmap.close();

  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob(
      (result) => (result ? resolve(result) : reject(new Error('Falha ao comprimir imagem'))),
      'image/jpeg',
      JPEG_QUALITY
    );
  });

  if (blob.size > MAX_BYTES) {
    throw new Error('Imagem demasiado grande — tenta uma foto mais pequena');
  }

  const dataUrl = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(new Error('Falha ao ler imagem'));
    reader.readAsDataURL(blob);
  });

  const base64 = dataUrl.split(',')[1];
  if (!base64) throw new Error('Falha ao ler imagem');
  return base64;
}
