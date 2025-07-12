use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{Result, Context};

const CALIDAD_JPG: u8 = 60;
const CALIDAD_WEBP: u8 = 60;
const LIMITE_WEBP_MB: u64 = 3 * 1024 * 1024; // 3MB en bytes

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let ruta = args.get(1).unwrap_or(&".".to_string()).clone();
    
    println!("📂 Carpeta objetivo: {}", ruta);
    
    verificar_herramientas()?;
    
    let tam_before = calcular_tamano_carpeta(&ruta)?;
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {wide_msg}")
        .unwrap());
    
    println!("🧼 Limpiando metadatos...");
    limpiar_metadatos(&ruta, &pb)?;
    
    println!("🔧 Optimizando JPG...");
    optimizar_jpg(&ruta, &pb)?;
    
    println!("🧊 Optimizando PNG...");
    optimizar_png(&ruta, &pb)?;
    
    println!("🌀 Buscando imágenes .heic grandes para convertir...");
    let (heic_total, convertidos) = convertir_heic_grandes(&ruta, &pb)?;
    
    // Calcular tamaño después
    let tam_after = calcular_tamano_carpeta(&ruta)?;
    let ahorrado = tam_before - tam_after;
    let porcentaje = if tam_before > 0 {
        (ahorrado as f64 / tam_before as f64) * 100.0
    } else {
        0.0
    };
    
    // Resultados
    println!();
    println!("✅ Optimización completada.");
    println!("📏 Tamaño antes   : {} KB", tam_before / 1024);
    println!("📉 Tamaño después : {} KB", tam_after / 1024);
    println!("💾 Espacio ahorrado: {} KB ({:.2}%)", ahorrado / 1024, porcentaje);
    println!("🖼️  Imágenes HEIC encontradas: {}", heic_total);
    println!("🔁 Imágenes HEIC convertidas a WebP: {}", convertidos);
    
    println!();
    println!("💡 WebP es ideal para fotos web, redes sociales y proyectos que requieren alto ahorro sin perder calidad visual.");
    
    Ok(())
}

fn verificar_herramientas() -> Result<()> {
    let herramientas = vec!["jpegoptim", "optipng", "exiftool", "cwebp", "heif-convert"];
    
    for herramienta in herramientas {
        let output = Command::new("which").arg(herramienta).output();
        if output.is_err() || !output.unwrap().status.success() {
            anyhow::bail!("❌ Falta '{}'. Instálalo con: sudo apt install jpegoptim optipng libimage-exiftool-perl webp libheif-examples", herramienta);
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

fn limpiar_metadatos(ruta: &str, pb: &ProgressBar) -> Result<()> {
    let mut contador = 0;
    
    for entry in WalkDir::new(ruta)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        if es_imagen(path) {
            pb.set_message(format!("Limpiando metadatos: {}", path.display()));
            
            let output = Command::new("exiftool")
                .args(&["-overwrite_original", "-all=", path.to_str().unwrap()])
                .output();
            
            if output.is_ok() {
                contador += 1;
            }
        }
    }
    
    pb.finish_with_message(format!("✅ Metadatos limpiados en {} archivos", contador));
    Ok(())
}

fn optimizar_jpg(ruta: &str, pb: &ProgressBar) -> Result<()> {
    let mut contador = 0;
    
    for entry in WalkDir::new(ruta)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        if es_imagen_jpg(path) {
            pb.set_message(format!("Optimizando JPG: {}", path.display()));
            
            let output = Command::new("jpegoptim")
                .args(&["--strip-all", "--max", &CALIDAD_JPG.to_string(), "--quiet", path.to_str().unwrap()])
                .output();
            
            if output.is_ok() {
                contador += 1;
            }
        }
    }
    
    pb.finish_with_message(format!("✅ JPG optimizados: {} archivos", contador));
    Ok(())
}

fn optimizar_png(ruta: &str, pb: &ProgressBar) -> Result<()> {
    let mut contador = 0;
    
    for entry in WalkDir::new(ruta)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        if es_imagen_png(path) {
            pb.set_message(format!("Optimizando PNG: {}", path.display()));
            
            let output = Command::new("optipng")
                .args(&["-o7", "-quiet", path.to_str().unwrap()])
                .output();
            
            if output.is_ok() {
                contador += 1;
            }
        }
    }
    
    pb.finish_with_message(format!("✅ PNG optimizados: {} archivos", contador));
    Ok(())
}

fn convertir_heic_grandes(ruta: &str, pb: &ProgressBar) -> Result<(u32, u32)> {
    let mut heic_total = 0;
    let mut convertidos = 0;
    
    for entry in WalkDir::new(ruta)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        if es_imagen_heic(path) {
            let metadata = fs::metadata(path)?;
            if metadata.len() > LIMITE_WEBP_MB {
                heic_total += 1;
                pb.set_message(format!("Convirtiendo HEIC: {}", path.display()));
                
                let nombre_sin_ext = path.with_extension("");
                let webp_out = nombre_sin_ext.with_extension("webp");
                let temp_jpg = nombre_sin_ext.with_extension("jpg");
                
                // Convertir HEIC a JPG temporal
                let heif_output = Command::new("heif-convert")
                    .args(&[path.to_str().unwrap(), temp_jpg.to_str().unwrap()])
                    .output();
                
                if heif_output.is_ok() && temp_jpg.exists() {
                    // Convertir JPG a WebP
                    let webp_output = Command::new("cwebp")
                        .args(&["-q", &CALIDAD_WEBP.to_string(), temp_jpg.to_str().unwrap(), "-o", webp_out.to_str().unwrap()])
                        .output();
                    
                    if webp_output.is_ok() && webp_out.exists() {
                        // Eliminar archivos temporales y original
                        let _ = fs::remove_file(&temp_jpg);
                        let _ = fs::remove_file(path);
                        convertidos += 1;
                    }
                }
            }
        }
    }
    
    pb.finish_with_message(format!("✅ HEIC convertidos: {} de {}", convertidos, heic_total));
    Ok((heic_total, convertidos))
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
