# Cleaner_photo - Limpiador_fotos

## ¿Qué hace? / What does it do?

Limpia los metadatos de tus fotos y reduce la calidad de las imágenes JPG al 60%, optimizando y ahorrando espacio de manera masiva. Convierte imágenes HEIC grandes a WebP para ahorrar aún más espacio. Ideal para carpetas con muchas fotos.

It cleans photo metadata and reduces JPG image quality to 60%, optimizing and saving disk space in bulk. It also converts large HEIC images to WebP for even more savings. Designed to run on folders with many photos.

## Uso / Usage

**Español:**
1. Instala las dependencias necesarias: `jpegoptim`, `optipng`, `exiftool`, `cwebp`, `heif-convert`.
2. Compila el proyecto con `cargo build --release`.
3. Ejecuta el programa indicando la carpeta objetivo (opcional, por defecto es la actual):
