use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use anyhow::{Result, Context};
use wait_timeout::ChildExt;

const CALIDAD_JPG: u8 = 50; // Calidad 
const CALIDAD_WEBP: u8 = 50;
const LIMITE_WEBP_MB: u64 = 3 * 1024 * 1024; 
const CRF_VIDEO: &str = "28"; // Calidad para ffmpeg (más alto = más compresión)
const PRESET_VIDEO: &str = "ultrafast";
const TIMEOUT_SECS: u64 = 10; // Timeout de 10 segundos por archivo

fn main() -> Result<()> {
    // Configurar manejo de Ctrl+C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\n🛑 Cancelando operación...");
        r.store(false, Ordering::SeqCst);
    }).expect("Error configurando Ctrl+C");

    let args: Vec<String> = env::args().collect();
    let ruta = if args.len() > 2 && args[1] == "--limpiar" {
        args.get(2).unwrap_or(&".".to_string()).clone()
    } else {
        args.get(1).unwrap_or(&".".to_string()).clone()
    };

    if args.len() > 1 && args[1] == "--limpiar" {
        limpiar_archivos_temporales(&ruta)?;
        println!("🧹 Limpieza de archivos temporales completada en: {}", ruta);
        return Ok(());
    }

    println!("📂 Carpeta objetivo: {}", ruta);
    
    verificar_herramientas()?;
    
    // Contar archivos totales primero
    println!("🔍 Contando archivos...");
    let archivos = recolectar_archivos(&ruta)?;
    let total_archivos = archivos.len();
    
    if total_archivos == 0 {
        println!("❌ No se encontraron imágenes ni videos en la carpeta.");
        return Ok(());
    }
    
    println!("📊 Total de archivos a procesar: {}", total_archivos);
    
    let tam_before = calcular_tamano_carpeta(&ruta)?;
    
    // Crear barra de progreso principal
    let mp = MultiProgress::new();
    let pb_principal = mp.add(ProgressBar::new(total_archivos as u64));
    pb_principal.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
        .unwrap()
        .progress_chars("#>-")
    );
    
    let pb_detalle = mp.add(ProgressBar::new_spinner());
    pb_detalle.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {wide_msg}")
        .unwrap());
    
    let mut contador_metadatos = 0;
    let mut contador_jpg = 0;
    let mut contador_png = 0;
    let mut heic_total = 0;
    let mut convertidos = 0;
    let mut videos_total = 0;
    let mut videos_optimizados = 0;
    let mut videos_ahorro_kb = 0u64;
    
    // Procesar archivos con progreso
    for (i, path) in archivos.iter().enumerate() {
        // Verificar si se canceló
        if !running.load(Ordering::SeqCst) {
            println!("\n⏹️  Operación cancelada por el usuario.");
            mostrar_resultados_parciales(contador_metadatos, contador_jpg, contador_png, heic_total, convertidos, videos_total, videos_optimizados, videos_ahorro_kb, tam_before, &ruta);
            return Ok(());
        }
        
        pb_principal.set_position(i as u64);
        pb_detalle.set_message(format!("Procesando: {}", path.file_name().unwrap().to_string_lossy()));
        
        // Limpiar metadatos
        if es_imagen(path) {
            if limpiar_metadatos_archivo(path).is_ok() {
                contador_metadatos += 1;
            }
        }
        
        // Optimizar JPG
        if es_imagen_jpg(path) {
            if optimizar_jpg_archivo(path).is_ok() {
                contador_jpg += 1;
            }
        }
        
        // Optimizar PNG
        if es_imagen_png(path) {
            if optimizar_png_archivo(path).is_ok() {
                contador_png += 1;
            }
        }
        
        // Convertir HEIC
        if es_imagen_heic(path) {
            let metadata = fs::metadata(path);
            if let Ok(meta) = metadata {
                if meta.len() > LIMITE_WEBP_MB {
                    heic_total += 1;
                    if convertir_heic_archivo(path).is_ok() {
                        convertidos += 1;
                    }
                }
            }
        }

        // Optimizar videos MP4/MOV
        if es_video(path) {
            videos_total += 1;
            match optimizar_video_archivo(path) {
                Ok(ahorro) => {
                    if ahorro > 0 {
                        videos_optimizados += 1;
                        videos_ahorro_kb += ahorro / 1024;
                    }
                },
                Err(e) => {
                    eprintln!("Error optimizando video {}: {}", path.display(), e);
                }
            }
        }
        
        // Actualizar mensaje principal
        pb_principal.set_message(format!("Metadatos: {} | JPG: {} | PNG: {} | HEIC: {}/{} | Videos: {}/{}", 
            contador_metadatos, contador_jpg, contador_png, convertidos, heic_total, videos_optimizados, videos_total));
    }
    
    pb_principal.finish_with_message("✅ Procesamiento completado");
    pb_detalle.finish_with_message("✅ Optimización finalizada");
    
    // Calcular resultados finales
    let tam_after = calcular_tamano_carpeta(&ruta)?;
    let ahorrado = tam_before - tam_after;
    let porcentaje = if tam_before > 0 {
        (ahorrado as f64 / tam_before as f64) * 100.0
    } else {
        0.0
    };
    
    // Mostrar resultados
    println!();
    println!("✅ Optimización completada.");
    println!("📏 Tamaño antes   : {:.2} MB", tam_before as f64 / 1024.0 / 1024.0);
    println!("📉 Tamaño después : {:.2} MB", tam_after as f64 / 1024.0 / 1024.0);
    println!("💾 Espacio ahorrado: {:.2} MB ({:.2}%)", ahorrado as f64 / 1024.0 / 1024.0, porcentaje);
    println!("🖼️  Metadatos limpiados: {}", contador_metadatos);
    println!("🔧 JPG optimizados: {}", contador_jpg);
    println!("🧊 PNG optimizados: {}", contador_png);
    println!("🖼️  Imágenes HEIC encontradas: {}", heic_total);
    println!("🔁 Imágenes HEIC convertidas a WebP: {}", convertidos);
    println!("🎬 Videos encontrados: {}", videos_total);
    println!("🎬 Videos optimizados: {} ({:.2} MB ahorrados)", videos_optimizados, videos_ahorro_kb as f64 / 1024.0);
    
    println!();
    println!("💡 WebP es ideal para fotos web, redes sociales y proyectos que requieren alto ahorro sin perder calidad visual.");
    println!("💡 ffmpeg permite recomprimir videos para ahorrar espacio sin perder mucha calidad.");
    
    // Al final, limpiar archivos temporales residuales
    limpiar_archivos_temporales(&ruta)?;
    println!("🧹 Limpieza de archivos temporales completada en: {}", ruta);

    Ok(())
}

fn mostrar_resultados_parciales(metadatos: u32, jpg: u32, png: u32, heic_total: u32, convertidos: u32, videos_total: u32, videos_optimizados: u32, videos_ahorro_kb: u64, tam_before: u64, ruta: &str) {
    println!("\n📊 Resultados parciales:");
    println!("🖼️  Metadatos limpiados: {}", metadatos);
    println!("🔧 JPG optimizados: {}", jpg);
    println!("🧊 PNG optimizados: {}", png);
    println!("🖼️  HEIC encontrados: {}", heic_total);
    println!("🔁 HEIC convertidos: {}", convertidos);
    println!("🎬 Videos encontrados: {}", videos_total);
    println!("🎬 Videos optimizados: {} ({:.2} MB ahorrados)", videos_optimizados, videos_ahorro_kb as f64 / 1024.0);
    
    // Calcular tamaño actual
    if let Ok(tam_actual) = calcular_tamano_carpeta(ruta) {
        let ahorrado = tam_before - tam_actual;
        let porcentaje = if tam_before > 0 {
            (ahorrado as f64 / tam_before as f64) * 100.0
        } else {
            0.0
        };
        println!("💾 Espacio ahorrado hasta ahora: {:.2} MB ({:.2}%)", ahorrado as f64 / 1024.0 / 1024.0, porcentaje);
    }
}

fn recolectar_archivos(ruta: &str) -> Result<Vec<std::path::PathBuf>> {
    let mut archivos = Vec::new();
    
    for entry in WalkDir::new(ruta)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        if es_imagen(path) || es_video(path) {
            archivos.push(path.to_path_buf());
        }
    }
    
    Ok(archivos)
}

fn verificar_herramientas() -> Result<()> {
    let herramientas = vec!["jpegoptim", "optipng", "exiftool", "cwebp", "heif-convert", "ffmpeg"];
    
    for herramienta in herramientas {
        let output = Command::new("which").arg(herramienta).output();
        if output.is_err() || !output.unwrap().status.success() {
            anyhow::bail!("❌ Falta '{}'. Instálalo con: sudo apt install jpegoptim optipng libimage-exiftool-perl webp libheif-examples ffmpeg", herramienta);
        }
    }
    Ok(())
}

fn calcular_tamano_carpeta(ruta: &str) -> Result<u64> {
    let output = Command::new("du")
        .args(&["-sb", ruta])
        .output()
        .context("Error ejecutando du")?;
    
    let output_str = String::from_utf8(output.stdout)
        .context("Error convirtiendo output a string")?;
    
    let tamano: u64 = output_str
        .split_whitespace()
        .next()
        .context("No se pudo obtener el tamaño")?
        .parse()
        .context("Error parseando el tamaño")?;
    
    Ok(tamano)
}

fn limpiar_metadatos_archivo(path: &Path) -> Result<()> {
    let mut child = Command::new("exiftool")
        .args(&["-overwrite_original", "-all=", path.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Error lanzando exiftool")?;
    let timeout = Duration::from_secs(TIMEOUT_SECS);
    match child.wait_timeout(timeout).context("Error esperando exiftool")? {
        Some(status) if status.success() => Ok(()),
        Some(_) => Err(anyhow::anyhow!("exiftool falló en {}", path.display())),
        None => {
            let _ = child.kill();
            eprintln!("⏱️  Timeout limpiando metadatos: {}", path.display());
            Ok(())
        }
    }
}

fn optimizar_jpg_archivo(path: &Path) -> Result<()> {
    let mut child = Command::new("jpegoptim")
        .args(&["--strip-all", "--max", &CALIDAD_JPG.to_string(), "--quiet", path.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Error lanzando jpegoptim")?;
    let timeout = Duration::from_secs(TIMEOUT_SECS);
    match child.wait_timeout(timeout).context("Error esperando jpegoptim")? {
        Some(status) if status.success() => Ok(()),
        Some(_) => Err(anyhow::anyhow!("jpegoptim falló en {}", path.display())),
        None => {
            let _ = child.kill();
            eprintln!("⏱️  Timeout optimizando JPG: {}", path.display());
            Ok(())
        }
    }
}

fn optimizar_png_archivo(path: &Path) -> Result<()> {
    let mut child = Command::new("optipng")
        .args(&["-o1", "-quiet", path.to_str().unwrap()]) // -o1 para máxima velocidad
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Error lanzando optipng")?;
    let timeout = Duration::from_secs(TIMEOUT_SECS);
    match child.wait_timeout(timeout).context("Error esperando optipng")? {
        Some(status) if status.success() => Ok(()),
        Some(_) => Err(anyhow::anyhow!("optipng falló en {}", path.display())),
        None => {
            let _ = child.kill();
            eprintln!("⏱️  Timeout optimizando PNG: {}", path.display());
            Ok(())
        }
    }
}

fn convertir_heic_archivo(path: &Path) -> Result<()> {
    let nombre_sin_ext = path.with_extension("");
    let webp_out = nombre_sin_ext.with_extension("webp");
    let temp_jpg = nombre_sin_ext.with_extension("jpg");
    // Convertir HEIC a JPG temporal
    let mut child = Command::new("heif-convert")
        .args(&[path.to_str().unwrap(), temp_jpg.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Error lanzando heif-convert")?;
    let timeout = Duration::from_secs(TIMEOUT_SECS);
    let heif_ok = match child.wait_timeout(timeout).context("Error esperando heif-convert")? {
        Some(status) if status.success() && temp_jpg.exists() => true,
        Some(_) => false,
        None => {
            let _ = child.kill();
            eprintln!("⏱️  Timeout convirtiendo HEIC: {}", path.display());
            false
        }
    };
    if heif_ok {
        // Convertir JPG a WebP
        let mut child = Command::new("cwebp")
            .args(&["-q", &CALIDAD_WEBP.to_string(), temp_jpg.to_str().unwrap(), "-o", webp_out.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Error lanzando cwebp")?;
        let timeout = Duration::from_secs(TIMEOUT_SECS);
        let webp_ok = match child.wait_timeout(timeout).context("Error esperando cwebp")? {
            Some(status) if status.success() && webp_out.exists() => true,
            Some(_) => false,
            None => {
                let _ = child.kill();
                eprintln!("⏱️  Timeout convirtiendo a WebP: {}", path.display());
                false
            }
        };
        if webp_ok {
            let _ = fs::remove_file(&temp_jpg);
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

fn optimizar_video_archivo(path: &Path) -> Result<u64> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if ext != "mp4" && ext != "mov" {
        return Ok(0);
    }
    let original_size = fs::metadata(path)?.len();
    let temp_out = path.with_extension(format!("opt.{}", ext));
    // Comando ffmpeg para recomprimir, priorizando rapidez
    let mut child = Command::new("nice")
        .args(&["-n", "10", "ffmpeg",
                "-threads", "2",
                "-i", path.to_str().unwrap(),
                "-vcodec", "libx264",
                "-crf", "30",
                "-preset", "ultrafast",
                "-acodec", "aac",
                "-b:a", "128k",
                "-y",
                temp_out.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Error lanzando ffmpeg")?;
    let timeout = Duration::from_secs(TIMEOUT_SECS * 2); // 20 segundos para videos
    match child.wait_timeout(timeout).context("Error esperando ffmpeg")? {
        Some(status) if status.success() && temp_out.exists() => {
            let new_size = fs::metadata(&temp_out)?.len();
            if new_size < original_size {
                fs::rename(&temp_out, path)?;
                Ok(original_size - new_size)
            } else {
                let _ = fs::remove_file(&temp_out);
                Ok(0)
            }
        },
        Some(_) => {
            let _ = fs::remove_file(&temp_out);
            Err(anyhow::anyhow!("ffmpeg falló en {}", path.display()))
        },
        None => {
            let _ = child.kill();
            let _ = fs::remove_file(&temp_out);
            eprintln!("⏱️  Timeout optimizando video: {}", path.display());
            Ok(0)
        }
    }
}

fn es_imagen(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "heic"),
        None => false,
    }
}

fn es_imagen_jpg(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg"),
        None => false,
    }
}

fn es_imagen_png(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => ext.to_lowercase() == "png",
        None => false,
    }
}

fn es_imagen_heic(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => ext.to_lowercase() == "heic",
        None => false,
    }
}

fn es_video(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => matches!(ext.to_lowercase().as_str(), "mp4" | "mov"),
        None => false,
    }
}

fn limpiar_archivos_temporales(ruta: &str) -> Result<()> {
    use std::ffi::OsStr;
    let patrones = [
        ".opt.jpg", ".opt.jpeg", ".opt.png", ".opt.webp", ".opt.mp4", ".opt.mov", ".tmp.jpg", ".tmp.png", ".tmp.webp", ".tmp.mp4", ".tmp.mov"
    ];
    let mut borrados = 0;
    for entry in WalkDir::new(ruta)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        let fname = path.file_name().and_then(OsStr::to_str).unwrap_or("");
        if patrones.iter().any(|pat| fname.ends_with(pat)) {
            if fs::remove_file(path).is_ok() {
                borrados += 1;
                println!("🗑️  Borrado archivo residual: {}", path.display());
            }
        }
    }
    if borrados == 0 {
        println!("✅ No se encontraron archivos temporales residuales.");
    } else {
        println!("✅ Se eliminaron {} archivos temporales residuales.", borrados);
    }
    Ok(())
}
