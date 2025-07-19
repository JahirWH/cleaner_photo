<div align="center"><img src="title.png"></div>

#  Cleaner Photo - Optimizador de Imágenes y Videos

Un optimizador rápido y eficiente escrito en Rust que limpia metadatos, optimiza imágenes (JPG, PNG, HEIC) y videos (MP4, MOV) para ahorrar espacio de almacenamiento masivamente.

##  Características / Features

- 🖼️ **Optimización de imágenes**: JPG, PNG, HEIC → WebP
- 🎬 **Optimización de videos**: MP4, MOV con ffmpeg
- 🧼 **Limpieza de metadatos**: Elimina EXIF y otros metadatos
- ⚡ **Procesamiento rápido**: Timeout de 10s por archivo, optimizado para velocidad
- 📊 **Progreso en tiempo real**: Barra de progreso con estadísticas detalladas
- 🛑 **Cancelación segura**: Ctrl+C para detener y ver resultados parciales
- 🧹 **Limpieza automática**: Elimina archivos temporales residuales
- 🎯 **Configuración inteligente**: Prioriza velocidad sobre máxima compresión

##  Instalación

### Prerrequisitos

Instala las herramientas necesarias:

```bash
sudo apt install jpegoptim optipng libimage-exiftool-perl webp libheif-examples ffmpeg
```

### Compilación /Build

```bash
# Clonar el repositorio
git clone https://github.com/tu-usuario/cleaner_photo.git
cd cleaner_photo

# Compilar en modo release (más rápido)
cargo build --release
```

##  Uso / Usage


```bash
# Con cargo (dentro de la carpeta cleaner_photo copiada de github)
cargo run --release -- /ruta/a/tu/carpeta_a_optimizar
```

### Solo limpieza de archivos temporales

```bash
# Limpiar archivos residuales en la carpeta actual
./target/release/cleaner_photo --limpiar

# Limpiar archivos residuales en una carpeta específica
./target/release/cleaner_photo --limpiar /ruta/a/tu/carpeta
```

## ⚙️ Configuración

### Parámetros de optimización

- **JPG**: Calidad 50% (configurable en `CALIDAD_JPG`)
- **PNG**: Optimización rápida (`optipng -o1`)
- **HEIC**: Conversión a WebP para archivos >3MB
- **Videos**: CRF 30, preset ultrafast, 2 hilos
- **Timeout**: 10 segundos por imagen, 20 por video

### Personalización

Puedes modificar los valores en `src/main.rs`:

```rust
const CALIDAD_JPG: u8 = 50;        // Calidad JPG (0-100)
const CALIDAD_WEBP: u8 = 50;       // Calidad WebP (0-100)
const TIMEOUT_SECS: u64 = 10;      // Timeout en segundos
```

## 📊 Ejemplo de salida

```
📂 Carpeta objetivo: /home/usuario/fotos
🔍 Contando archivos...
📊 Total de archivos a procesar: 150
🧼 Limpiando metadatos...
🔧 Optimizando JPG...
🧊 Optimizando PNG...
🌀 Buscando imágenes .heic grandes para convertir...
🎬 Optimizando videos...

✅ Optimización completada.
📏 Tamaño antes   : 2.45 GB
📉 Tamaño después : 1.87 GB
💾 Espacio ahorrado: 0.58 GB (23.67%)
🖼️  Metadatos limpiados: 150
🔧 JPG optimizados: 120
🧊 PNG optimizados: 25
🖼️  Imágenes HEIC encontradas: 3
🔁 Imágenes HEIC convertidas a WebP: 3
🎬 Videos encontrados: 5
🎬 Videos optimizados: 4 (45.2 MB ahorrados)
🧹 Limpieza de archivos temporales completada
```

## 🛠️ Tecnologías utilizadas

- **Rust**: Lenguaje principal
- **ffmpeg**: Optimización de videos
- **jpegoptim**: Optimización de JPG
- **optipng**: Optimización de PNG
- **exiftool**: Limpieza de metadatos
- **heif-convert**: Conversión HEIC
- **cwebp**: Conversión a WebP

## 🔧 Solución de problemas

### Error: "Falta 'herramienta'"
```bash
sudo apt install jpegoptim optipng libimage-exiftool-perl webp libheif-examples ffmpeg
```

### Timeout en archivos grandes
- El programa automáticamente salta archivos que tardan más de 10 segundos
- Los archivos problemáticos se muestran en la consola

### Cancelación segura
- Presiona `Ctrl+C` para detener el proceso
- Se mostrarán los resultados parciales obtenidos hasta ese momento

## 📝 Licencia

Este proyecto está bajo la Licencia MIT. Ver el archivo `LICENSE` para más detalles.

## 🤝 Contribuciones

Las contribuciones son bienvenidas. Por favor:

1. Abre un fork, has el commit y abre un pull request


---
